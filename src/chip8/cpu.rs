use rand::Rng;

use super::mmu;

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;

// 레지스터 구현
#[derive(Default)]
pub struct Registers {
    // 참고: VF 레지스터는 일부 명령어의 플래그 역할을 하므로 사용하지 않는 것이 좋습니다.
    // 덧셈 연산에서는 VF가 캐리 플래그이고, 뺄셈에서는 "빌림 없음" 플래그입니다.
    // 그리기 명령어에서는 픽셀 충돌 시 VF가 설정됩니다.
    pub v: [u8; 16],
    // 인덱스 레지스터
    pub i: usize,
    // 프로그램 카운터
    pub pc: usize,
    // 스택 포인터
    pub sp: usize,
    // 이 타이머는 게임 이벤트의 타이밍을 측정하는 데 사용됩니다.
    // 값을 설정하고 읽을 수 있습니다.
    pub delay: u8,
    // 이 타이머는 음향 효과에 사용됩니다.
    // 값이 0이 아니면 삐 소리가 납니다.
    pub sound: u8,
}

// 키패드 구현
#[derive(Default)]
struct Keypad {
    pub state: [bool; 16],
    pub waiting: bool,
    pub register: usize,
}

// CPU 구현
pub struct CPU {
    pub mmu: mmu::MMU,

    pub vram: [u8; HEIGHT * WIDTH],
    vram_changed: bool,

    pub registers: Registers,
    // 스택은 서브루틴이 호출될 때 반환 주소를 저장하는 데만 사용됩니다.
    stack: [u16; 16],
    keypad: Keypad,

    rng: rand::rngs::ThreadRng,
}

impl CPU {
    pub fn new(mmu: mmu::MMU) -> Self {
        let mut cpu = CPU {
            mmu,
            vram: [0; HEIGHT * WIDTH],
            vram_changed: false,
            registers: Registers::default(),
            stack: [0; 16],
            keypad: Keypad::default(),
            rng: rand::rng(),
            // debug: false,
        };

        cpu.reset();
        cpu
    }

    pub fn fetch_instruction(&mut self) -> u16 {
        self.mmu.read_word(self.registers.pc)
    }

    pub fn step(&mut self, keypad: [bool; 16]) {
        self.vram_changed = false;
        self.keypad.state = keypad;

        if self.keypad.waiting {
            for i in 0..=15 {
                if self.keypad.state[i] {
                    self.keypad.waiting = false;
                    self.registers.v[self.keypad.register] = i as u8;
                    self.registers.pc += 2;
                    break;
                }
            }
        } else {
            let opcode = self.fetch_instruction();

            self.registers.pc += 2;

            self.execute(opcode);
        }
    }

    pub fn update_timers(&mut self) {
        if self.registers.delay > 0 {
            self.registers.delay -= 1;
        }

        if self.registers.sound > 0 {
            self.registers.sound -= 1;
        }
    }

    pub fn should_redraw(&self) -> bool {
        self.vram_changed
    }

    pub fn should_beep(&self) -> bool {
        self.registers.sound > 0
    }

    pub fn reset(&mut self) {
        self.mmu.reset();
        self.vram = [0; HEIGHT * WIDTH];
        self.vram_changed = false;
        self.registers = Registers {
            pc: mmu::ROM_BASE_ADDR,
            ..Registers::default()
        };
        self.stack = [0; 16];
        self.keypad = Keypad::default();
    }

    pub fn read_byte(&mut self, addr: u16) -> u8 {
        self.mmu.read_byte(addr as usize)
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.reset();
        self.mmu.load_rom(rom);
    }

    fn execute(&mut self, opcode: u16) {
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;

        match opcode & 0xF000 {
            0x0000 => match opcode & 0x000F {
                // disp_clear() (0x00E0)
                0x0000 => {
                    for x in 0..WIDTH {
                        for y in 0..HEIGHT {
                            self.vram[x + (y * WIDTH)] = 0;
                        }
                    }
                    self.vram_changed = true;
                },
                // return (0x00EE)
                0x000E => {
                    self.registers.sp -= 1;
                    self.registers.pc = self.stack[self.registers.sp] as usize;
                },
                _ => self.unsupported_opcode(opcode),
            },
            // goto NNN;
            0x1000 => {
                self.registers.pc = (opcode & 0x0FFF) as usize;
            },
            // *(0xNNN) ()
            0x2000 => {
                self.stack[self.registers.sp] = self.registers.pc as u16;
                self.registers.sp += 1;
                self.registers.pc = (opcode & 0x0FFF) as usize;
            },
            // if (Vx == NN)
            0x3000 => {
                let val = (opcode & 0x00FF) as u8;

                if self.registers.v[x] ==  val {
                    self.registers.pc += 2;
                }
            },
            // if (Vx != NN)
            0x4000 => {
                let val = (opcode & 0x00FF) as u8;

                if self.registers.v[x] != val {
                    self.registers.pc += 2;
                }
            }
            // if (Vx == Vy)
            0x5000 => {
                if self.registers.v[x] == self.registers.v[y] {
                    self.registers.pc += 2;
                }
            },
            // Vx = NN
            0x6000 => {
                self.registers.v[x] = (opcode & 0x00FF) as u8;
            },
            // Vx += NN
            0x7000 => {
                let val = self.registers.v[x] as u16 + (opcode & 0x00FF) as u16;
                self.registers.v[x] = val as u8;
            }
            0x8000 => self.op_8xy(opcode, x, y),
            // if (Vx != Vy)
            0x9000 => {
                if self.registers.v[x] != self.registers.v[y] {
                    self.registers.pc += 2;
                }
            },
            // I = NNN
            0xA000 => {
                self.registers.i = (opcode & 0x0FFF) as usize;
            },
            // PC = V0 + NNN
            0xB000 => {
                self.registers.pc = (self.registers.v[0] as u16 + (opcode & 0x0FFF)) as usize;
            },
            // Vx = rand() & NN
            0xC000 => {
                self.registers.v[x] = self.rng.random::<u8>() & (opcode & 0x00FF) as u8;
            },
            // draw(Vx, Vy, N)
            0xD000 => {
                let height = (opcode & 0x000F) as usize;
                let vx = self.registers.v[x] as usize;
                let vy = self.registers.v[y] as usize;

                self.registers.v[0xF] = 0;

                for yline in 0..height {
                    let pixel = self.mmu.read_byte(self.registers.i + yline);

                    for xline in 0..8 {
                        let vram_x = (vx + xline) % WIDTH;
                        let vram_y = (vy + yline) % HEIGHT;

                        if (pixel & (0x80 >> xline)) != 0 {
                            if self.vram[vram_x + vram_y * WIDTH] == 1 {
                                self.registers.v[0xF] |= 1;
                            }

                            self.vram[vram_x + vram_y * WIDTH] ^= 1;
                        }
                    }
                }

                self.vram_changed = true;
            },
            0xE000 => self.op_ex(opcode, x),
            0xF000 => self.op_fx(opcode, x),
            _ => self.unsupported_opcode(opcode),
        };
    }

    fn op_8xy(&mut self, opcode: u16, x: usize, y: usize) {
        match opcode & 0x000F {
            // Vx = Vy
            0 => {
                self.registers.v[x] = self.registers.v[y];
            },
            // Vx = Vx | Vy
            1 => {
                self.registers.v[x] |= self.registers.v[y];
            },
            // Vx = Vx & Vy
            2 => {
                self.registers.v[x] &= self.registers.v[y];
            },
            // Vx = Vx ^ Vy
            3 => {
                self.registers.v[x] ^= self.registers.v[y];
            },
            // Vx = Vx + Vy
            4 => {
                let val
                    = self.registers.v[x] as u16 + self.registers.v[y] as u16;

                self.registers.v[x] = val as u8;
                self.registers.v[0xF] = if val > 0xFF { 1 } else { 0 };
            },
            // Vx = Vx - Vy
            5 => {
                self.registers.v[0xF] = if self.registers.v[x] > self.registers.v[y] {
                    1
                } else {
                    0
                };
                self.registers.v[x] = self.registers.v[x].wrapping_sub(self.registers.v[y]);
            },
            // Vx - Vx >> 1
            6 => {
                self.registers.v[0xF] = self.registers.v[x] & 1;
                self.registers.v[x] >>= 1;
            },
            // Vx = Vy - Vx
            7 => {
                self.registers.v[0xF] = if self.registers.v[x] > self.registers.v[y] {
                    0
                } else {
                    1
                };
                self.registers.v[x] = self.registers.v[y].wrapping_sub(self.registers.v[x]);
            },
            // Vx = Vx << 1
            0xE => {
                self.registers.v[0xF] = (self.registers.v[x] >> 7) & 1;
                self.registers.v[x] <<= 1;
            },
            _ => self.unsupported_opcode(opcode),
        };
    }

    fn op_ex(&mut self, opcode: u16, x: usize) {
        match (opcode & 0x00FF) as u8 {
            // if (key() == Vx)
            0x009E => {
                if self.keypad.state[self.registers.v[x] as usize] {
                    self.registers.pc += 2;
                }
            },
            // if (key() == Vx)
            0x00A1 => {
                if !self.keypad.state[self.registers.v[x] as usize] {
                    self.registers.pc += 2;
                }
            },
            _ => self.unsupported_opcode(opcode),
        };
    }

    fn op_fx(&mut self, opcode: u16, x: usize) {
        match (opcode & 0x00FF) as u8 {
            // Vx = get_delay()
            0x0007 => {
                self.registers.v[x] = self.registers.delay;
            },
            // Vx = get_key()
            0x000A => {
                self.keypad.waiting = true;
                self.keypad.register = x;
            },
            // delay_timer(Vx)
            0x0015 => {
                self.registers.delay = self.registers.v[x];
            },
            // sound_timer(Vx)
            0x0018 => {
                self.registers.sound = self.registers.v[x];
            },
            // I += Vx
            0x001E => {
                self.registers.i += self.registers.v[x] as usize;
                self.registers.v[0xF] = if self.registers.i > 0x0F00 { 1 } else { 0 };
            },
            // I = sprite_addr[Vx]
            0x0029 => {
                self.registers.i = mmu::FONT_BASE_ADDR + (self.registers.v[x] as usize) * 5;
            },
            // set_BCD(Vx)
            // *(I+0) = BCD(3)
            // *(I+1) = BCD(2)
            // *(I+2) = BCD(1)
            0x0033 => {
                let val = self.registers.v[x];

                self.mmu.write_byte(self.registers.i, val / 100);
                self.mmu.write_byte(self.registers.i + 1, (val % 100) / 10);
                self.mmu.write_byte(self.registers.i + 2, val % 10);
            },
            // reg_dump(Vx, &I)
            0x0055 => {
                for i in 0..=x {
                    self.mmu.write_byte(self.registers.i + i, self.registers.v[i]);
                }
            },
            // reg_load(Vx, &I)
            0x0065 => {
                for i in 0..=x {
                    self.registers.v[i] = self.mmu.read_byte(self.registers.i + i);
                }
            },
            _ => self.unsupported_opcode(opcode),
        };
    }

    fn unsupported_opcode(&self, opcode: u16) {
        panic!(
            "unsupported opcode 0x{:04X} @ ${:04X}\n",
            opcode,
            self.registers.pc
        );
    }
}