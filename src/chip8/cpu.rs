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
            let old_pc = self.registers.pc;
            let opcode = self.fetch_instruction();
            self.execute(opcode);

            if self.keypad.waiting {
                self.registers.pc = old_pc;
            }
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

    // 테스트를 위한 public 메서드들
    pub fn get_keypad_state(&self) -> &[bool; 16] {
        &self.keypad.state
    }

    pub fn is_keypad_waiting(&self) -> bool {
        self.keypad.waiting
    }

    pub fn get_stack(&self) -> &[u16; 16] {
        &self.stack
    }

    pub fn update_keypad_state(&mut self, keypad: [bool; 16]) {
        self.keypad.state = keypad;
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
                    self.registers.pc += 2;
                    return;
                },
                // return (0x00EE)
                0x000E => {
                    self.registers.sp -= 1;
                    self.registers.pc = self.stack[self.registers.sp] as usize;
                    return;
                },
                _ => self.unsupported_opcode(opcode),
            },
            // goto NNN;
            0x1000 => {
                self.registers.pc = (opcode & 0x0FFF) as usize;
                return;
            },
            // *(0xNNN) ()
            0x2000 => {
                // 현재 PC+2(다음 명령어 주소)를 스택에 저장
                self.stack[self.registers.sp] = (self.registers.pc + 2) as u16;
                self.registers.sp += 1;
                self.registers.pc = (opcode & 0x0FFF) as usize;
                return;
            },
            // if (Vx == NN)
            0x3000 => {
                let val = (opcode & 0x00FF) as u8;
                if self.registers.v[x] ==  val {
                    self.registers.pc += 4;
                } else {
                    self.registers.pc += 2;
                }
                return;
            },
            // if (Vx != NN)
            0x4000 => {
                let val = (opcode & 0x00FF) as u8;
                if self.registers.v[x] != val {
                    self.registers.pc += 4;
                } else {
                    self.registers.pc += 2;
                }
                return;
            },
            // if (Vx == Vy)
            0x5000 => {
                if (opcode & 0x000F) == 0 {
                    if self.registers.v[x] == self.registers.v[y] {
                        self.registers.pc += 4;
                    } else {
                        self.registers.pc += 2;
                    }
                    return;
                } else {
                    self.unsupported_opcode(opcode);
                }
            },
            // Vx = NN
            0x6000 => {
                self.registers.v[x] = (opcode & 0x00FF) as u8;
                self.registers.pc += 2;
            },
            // Vx += NN
            0x7000 => {
                let val = self.registers.v[x] as u16 + (opcode & 0x00FF) as u16;
                self.registers.v[x] = val as u8;
                self.registers.pc += 2;
            },
            0x8000 => {
                self.op_8xy(opcode, x, y);
                self.registers.pc += 2;
            },
            // if (Vx != Vy)
            0x9000 => {
                if self.registers.v[x] != self.registers.v[y] {
                    self.registers.pc += 4;
                } else {
                    self.registers.pc += 2;
                }
                return;
            },
            // I = NNN
            0xA000 => {
                self.registers.i = (opcode & 0x0FFF) as usize;
                self.registers.pc += 2;
            },
            // PC = V0 + NNN
            0xB000 => {
                self.registers.pc = (self.registers.v[0] as u16 + (opcode & 0x0FFF)) as usize;
                return;
            },
            // Vx = rand() & NN
            0xC000 => {
                self.registers.v[x] = self.rng.random::<u8>() & (opcode & 0x00FF) as u8;
                self.registers.pc += 2;
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
                self.registers.pc += 2;
            },
            0xE000 => {
                match opcode & 0x00FF {
                    0x9E => {
                        // if (key() == Vx)
                        if self.keypad.state[self.registers.v[x] as usize] {
                            self.registers.pc += 4;
                        } else {
                            self.registers.pc += 2;
                        }
                        return;
                    },
                    0xA1 => {
                        // if (key() != Vx)
                        if !self.keypad.state[self.registers.v[x] as usize] {
                            self.registers.pc += 4;
                        } else {
                            self.registers.pc += 2;
                        }
                        return;
                    },
                    _ => self.unsupported_opcode(opcode),
                }
            },
            0xF000 => {
                self.op_fx(opcode, x);
                self.registers.pc += 2;
            },
            _ => self.unsupported_opcode(opcode),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chip8::mmu::MMU;

    fn create_cpu_with_rom(rom_data: Vec<u8>) -> CPU {
        let mmu = MMU::new(rom_data);
        CPU::new(mmu)
    }

    #[test]
    fn test_cpu_initialization() {
        let cpu = create_cpu_with_rom(vec![]);
        
        assert_eq!(cpu.registers.pc, mmu::ROM_BASE_ADDR);
        assert_eq!(cpu.registers.sp, 0);
        assert_eq!(cpu.registers.i, 0);
        assert_eq!(cpu.registers.delay, 0);
        assert_eq!(cpu.registers.sound, 0);
        assert_eq!(cpu.vram.iter().sum::<u8>(), 0);
    }

    #[test]
    fn test_clear_display() {
        // 화면을 먼저 채우고
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.vram[0] = 1;
        cpu.vram[100] = 1;
        
        // 0x00E0 명령어 실행 (화면 클리어)
        cpu.mmu.write_byte(0x200, 0x00);
        cpu.mmu.write_byte(0x201, 0xE0);
        
        cpu.step([false; 16]);
        
        assert!(cpu.should_redraw());
        assert_eq!(cpu.vram.iter().sum::<u8>(), 0);
    }

    #[test]
    fn test_return() {
        let mut cpu = create_cpu_with_rom(vec![]);
        
        // 스택에 반환 주소 설정
        cpu.stack[0] = 0x300;
        cpu.registers.sp = 1;
        cpu.registers.pc = 0x200;
        
        // 0x00EE 명령어 실행 (return)
        cpu.mmu.write_byte(0x200, 0x00);
        cpu.mmu.write_byte(0x201, 0xEE);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.pc, 0x300);
        assert_eq!(cpu.registers.sp, 0);
    }

    #[test]
    fn test_goto() {
        let mut cpu = create_cpu_with_rom(vec![]);
        
        // 0x1ABC 명령어 실행 (goto 0xABC)
        cpu.mmu.write_byte(0x200, 0x1A);
        cpu.mmu.write_byte(0x201, 0xBC);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.pc, 0xABC);
    }

    #[test]
    fn test_call() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.pc = 0x200;
        
        // 0x2ABC 명령어 실행 (call 0xABC)
        cpu.mmu.write_byte(0x200, 0x2A);
        cpu.mmu.write_byte(0x201, 0xBC);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.pc, 0xABC);
        assert_eq!(cpu.registers.sp, 1);
        assert_eq!(cpu.stack[0], 0x202);
    }

    #[test]
    fn test_skip_if_equal() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[0] = 0x42;
        cpu.registers.pc = 0x200;
        
        // 0x3042 명령어 실행 (if V0 == 0x42)
        cpu.mmu.write_byte(0x200, 0x30);
        cpu.mmu.write_byte(0x201, 0x42);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.pc, 0x204); // 2바이트 건너뜀
    }

    #[test]
    fn test_skip_if_not_equal() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[0] = 0x42;
        cpu.registers.pc = 0x200;
        
        // 0x4043 명령어 실행 (if V0 != 0x43)
        cpu.mmu.write_byte(0x200, 0x40);
        cpu.mmu.write_byte(0x201, 0x43);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.pc, 0x204); // 2바이트 건너뜀
    }

    #[test]
    fn test_skip_if_registers_equal() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[1] = 0x42;
        cpu.registers.v[2] = 0x42;
        cpu.registers.pc = 0x200;
        
        // 0x5120 명령어 실행 (if V1 == V2)
        cpu.mmu.write_byte(0x200, 0x51);
        cpu.mmu.write_byte(0x201, 0x20);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.pc, 0x204); // 2바이트 건너뜀
    }

    #[test]
    fn test_set_register() {
        let mut cpu = create_cpu_with_rom(vec![]);
        
        // 0x6042 명령어 실행 (V0 = 0x42)
        cpu.mmu.write_byte(0x200, 0x60);
        cpu.mmu.write_byte(0x201, 0x42);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.v[0], 0x42);
    }

    #[test]
    fn test_add_to_register() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[0] = 0x10;
        
        // 0x7020 명령어 실행 (V0 += 0x20)
        cpu.mmu.write_byte(0x200, 0x70);
        cpu.mmu.write_byte(0x201, 0x20);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.v[0], 0x30);
    }

    #[test]
    fn test_register_operations() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[1] = 0x0F;
        cpu.registers.v[2] = 0xF0;
        
        // 0x8120 명령어 실행 (V1 = V2)
        cpu.mmu.write_byte(0x200, 0x81);
        cpu.mmu.write_byte(0x201, 0x20);
        
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.v[1], 0xF0);
        
        // 0x8121 명령어 실행 (V1 = V1 | V2)
        cpu.mmu.write_byte(0x202, 0x81);
        cpu.mmu.write_byte(0x203, 0x21);
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.v[1], 0xF0);
        
        // 0x8122 명령어 실행 (V1 = V1 & V2)
        cpu.mmu.write_byte(0x204, 0x81);
        cpu.mmu.write_byte(0x205, 0x22);
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.v[1], 0xF0);
        
        // 0x8123 명령어 실행 (V1 = V1 ^ V2)
        cpu.mmu.write_byte(0x206, 0x81);
        cpu.mmu.write_byte(0x207, 0x23);
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.v[1], 0x00);
    }

    #[test]
    fn test_arithmetic_operations() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[1] = 0x80;
        cpu.registers.v[2] = 0x80;
        
        // 0x8124 명령어 실행 (V1 = V1 + V2, 캐리 플래그 테스트)
        cpu.mmu.write_byte(0x200, 0x81);
        cpu.mmu.write_byte(0x201, 0x24);
        
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.v[1], 0x00);
        assert_eq!(cpu.registers.v[0xF], 1); // 캐리 플래그
        
        // 0x8125 명령어 실행 (V1 = V1 - V2, 빌림 플래그 테스트)
        cpu.registers.v[1] = 0x80;
        cpu.registers.v[2] = 0x40;
        cpu.mmu.write_byte(0x202, 0x81);
        cpu.mmu.write_byte(0x203, 0x25);
        
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.v[1], 0x40);
        assert_eq!(cpu.registers.v[0xF], 1); // 빌림 없음 플래그
    }

    #[test]
    fn test_shift_operations() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[1] = 0x85; // 10000101
        
        // 0x8126 명령어 실행 (V1 = V1 >> 1)
        cpu.mmu.write_byte(0x200, 0x81);
        cpu.mmu.write_byte(0x201, 0x26);
        
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.v[1], 0x42);
        assert_eq!(cpu.registers.v[0xF], 1); // LSB가 1이었으므로
        
        // 0x812E 명령어 실행 (V1 = V1 << 1)
        cpu.mmu.write_byte(0x202, 0x81);
        cpu.mmu.write_byte(0x203, 0x2E);
        
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.v[1], 0x84);
        assert_eq!(cpu.registers.v[0xF], 0); // MSB가 0이었으므로
    }

    #[test]
    fn test_skip_if_registers_not_equal() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[1] = 0x42;
        cpu.registers.v[2] = 0x43;
        cpu.registers.pc = 0x200;
        
        // 0x9120 명령어 실행 (if V1 != V2)
        cpu.mmu.write_byte(0x200, 0x91);
        cpu.mmu.write_byte(0x201, 0x20);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.pc, 0x204); // 2바이트 건너뜀
    }

    #[test]
    fn test_set_index_register() {
        let mut cpu = create_cpu_with_rom(vec![]);
        
        // 0xAABC 명령어 실행 (I = 0xABC)
        cpu.mmu.write_byte(0x200, 0xAA);
        cpu.mmu.write_byte(0x201, 0xBC);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.i, 0xABC);
    }

    #[test]
    fn test_jump_with_offset() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[0] = 0x20;
        
        // 0xBABC 명령어 실행 (PC = V0 + 0xABC)
        cpu.mmu.write_byte(0x200, 0xBA);
        cpu.mmu.write_byte(0x201, 0xBC);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.registers.pc, 0xADC); // 0x20 + 0xABC
    }

    #[test]
    fn test_random() {
        let mut cpu = create_cpu_with_rom(vec![]);
        
        // 0xC0FF 명령어 실행 (V0 = rand() & 0xFF)
        cpu.mmu.write_byte(0x200, 0xC0);
        cpu.mmu.write_byte(0x201, 0xFF);
        
        cpu.step([false; 16]);
        
        // 랜덤 값이 설정되었는지 확인 (정확한 값은 예측할 수 없음)
        // u8 타입은 이미 0xFF 이하이므로 비교 불필요
    }

    #[test]
    fn test_draw() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[1] = 0; // x 좌표
        cpu.registers.v[2] = 0; // y 좌표
        cpu.registers.i = 0x300; // 스프라이트 주소
        
        // 스프라이트 데이터 설정 (8x1 픽셀)
        cpu.mmu.write_byte(0x300, 0xFF); // 모든 픽셀이 켜짐
        
        // 0xD121 명령어 실행 (draw V1, V2, 1)
        cpu.mmu.write_byte(0x200, 0xD1);
        cpu.mmu.write_byte(0x201, 0x21);
        
        cpu.step([false; 16]);
        
        assert!(cpu.should_redraw());
        // 첫 번째 행의 모든 픽셀이 켜져야 함
        for i in 0..8 {
            assert_eq!(cpu.vram[i], 1);
        }
    }

    #[test]
    fn test_key_operations() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.v[0] = 5; // 키 5를 확인할 값
        
        // 0xE09E 명령어 실행 (if key() == V0)
        cpu.mmu.write_byte(0x200, 0xE0);
        cpu.mmu.write_byte(0x201, 0x9E);
        
        // 키 5가 눌린 상태로 테스트
        let mut keypad = [false; 16];
        keypad[5] = true;
        
        cpu.step(keypad);
        assert_eq!(cpu.registers.pc, 0x204); // 2바이트 건너뜀
        
        // 0xE0A1 명령어 실행 (if key() != V0)
        cpu.mmu.write_byte(0x204, 0xE0);
        cpu.mmu.write_byte(0x205, 0xA1);
        
        // 키 5가 눌리지 않은 상태로 테스트
        let keypad = [false; 16];
        cpu.step(keypad);
        assert_eq!(cpu.registers.pc, 0x208); // 2바이트 건너뜀
    }

    #[test]
    fn test_timer_operations() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.delay = 50;
        cpu.registers.sound = 30;
        
        // 0xF007 명령어 실행 (V0 = get_delay())
        cpu.mmu.write_byte(0x200, 0xF0);
        cpu.mmu.write_byte(0x201, 0x07);
        
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.v[0], 50);
        
        // 0xF015 명령어 실행 (delay_timer(V0))
        cpu.registers.v[0] = 100;
        cpu.mmu.write_byte(0x202, 0xF0);
        cpu.mmu.write_byte(0x203, 0x15);
        
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.delay, 100);
        
        // 0xF018 명령어 실행 (sound_timer(V0))
        cpu.registers.v[0] = 75;
        cpu.mmu.write_byte(0x204, 0xF0);
        cpu.mmu.write_byte(0x205, 0x18);
        
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.sound, 75);
    }

    #[test]
    fn test_timer_decrement() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.delay = 10;
        cpu.registers.sound = 5;
        
        // 타이머 감소 테스트
        for _i in 0..10 {
            cpu.update_timers();
        }
        assert_eq!(cpu.registers.delay, 0);
        assert_eq!(cpu.registers.sound, 0);
    }

    #[test]
    fn test_sound_timer() {
        let mut cpu = create_cpu_with_rom(vec![]);
        
        assert!(!cpu.should_beep());
        
        cpu.registers.sound = 1;
        assert!(cpu.should_beep());
        
        cpu.update_timers();
        assert!(!cpu.should_beep());
    }

    #[test]
    fn test_index_operations() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.i = 0x100;
        cpu.registers.v[0] = 0x50;
        
        // 0xF01E 명령어 실행 (I += V0)
        cpu.mmu.write_byte(0x200, 0xF0);
        cpu.mmu.write_byte(0x201, 0x1E);
        
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.i, 0x150);
        
        // 0xF029 명령어 실행 (I = sprite_addr[V0])
        cpu.registers.v[0] = 5; // 숫자 5의 스프라이트
        cpu.mmu.write_byte(0x202, 0xF0);
        cpu.mmu.write_byte(0x203, 0x29);
        
        cpu.step([false; 16]);
        assert_eq!(cpu.registers.i, mmu::FONT_BASE_ADDR + 25); // 5 * 5 = 25
    }

    #[test]
    fn test_bcd_operation() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.i = 0x300;
        cpu.registers.v[0] = 123; // BCD로 변환할 값
        
        // 0xF033 명령어 실행 (set_BCD(V0))
        cpu.mmu.write_byte(0x200, 0xF0);
        cpu.mmu.write_byte(0x201, 0x33);
        
        cpu.step([false; 16]);
        
        assert_eq!(cpu.mmu.read_byte(0x300), 1); // 백의 자리
        assert_eq!(cpu.mmu.read_byte(0x301), 2); // 십의 자리
        assert_eq!(cpu.mmu.read_byte(0x302), 3); // 일의 자리
    }

    #[test]
    fn test_register_dump() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.i = 0x300;
        
        // 레지스터에 값 설정
        for i in 0..5 {
            cpu.registers.v[i] = i as u8 + 10;
        }
        
        // 0xF455 명령어 실행 (reg_dump(V4, &I))
        cpu.mmu.write_byte(0x200, 0xF4);
        cpu.mmu.write_byte(0x201, 0x55);
        
        cpu.step([false; 16]);
        
        // 메모리에 저장되었는지 확인
        for i in 0..5 {
            assert_eq!(cpu.mmu.read_byte(0x300 + i), i as u8 + 10);
        }
    }

    #[test]
    fn test_register_load() {
        let mut cpu = create_cpu_with_rom(vec![]);
        cpu.registers.i = 0x300;
        
        // 메모리에 테스트 데이터 설정
        for i in 0..5 {
            cpu.mmu.write_byte(0x300 + i, i as u8 + 10);
        }
        
        // 레지스터 초기화
        for i in 0..5 {
            cpu.registers.v[i] = 0;
        }
        
        // 0xF465 명령어 실행 (reg_load(V4, &I))
        cpu.mmu.write_byte(0x200, 0xF4);
        cpu.mmu.write_byte(0x201, 0x65);
        
        cpu.step([false; 16]);
        
        // 레지스터에 로드되었는지 확인
        for i in 0..5 {
            assert_eq!(cpu.registers.v[i], i as u8 + 10);
        }
    }

    #[test]
    fn test_wait_for_key() {
        let mut cpu = create_cpu_with_rom(vec![]);
        
        // 0xF00A 명령어 실행 (V0 = get_key())
        cpu.mmu.write_byte(0x200, 0xF0);
        cpu.mmu.write_byte(0x201, 0x0A);
        
        cpu.step([false; 16]);
        
        // 키 대기 상태인지 확인
        assert_eq!(cpu.registers.pc, 0x200); // PC가 증가하지 않음
        
        // 키가 눌렸을 때
        let mut keypad = [false; 16];
        keypad[7] = true; // 키 7 누름
        
        cpu.step(keypad);
        
        assert_eq!(cpu.registers.v[0], 7);
        assert_eq!(cpu.registers.pc, 0x202); // PC가 증가함
    }

    #[test]
    fn test_cpu_reset() {
        let mut cpu = create_cpu_with_rom(vec![0x12, 0x34, 0x56, 0x78]);
        
        // CPU 상태 변경
        cpu.registers.v[0] = 0xFF;
        cpu.registers.i = 0x1000;
        cpu.registers.delay = 50;
        cpu.registers.sound = 30;
        cpu.vram[0] = 1;
        cpu.vram[100] = 1;
        
        // 리셋
        cpu.reset();
        
        // 모든 상태가 초기화되었는지 확인
        assert_eq!(cpu.registers.pc, mmu::ROM_BASE_ADDR);
        assert_eq!(cpu.registers.sp, 0);
        assert_eq!(cpu.registers.i, 0);
        assert_eq!(cpu.registers.delay, 0);
        assert_eq!(cpu.registers.sound, 0);
        assert_eq!(cpu.vram.iter().sum::<u8>(), 0);
        assert_eq!(cpu.registers.v[0], 0);
    }

    #[test]
    fn test_rom_loading() {
        let mut cpu = create_cpu_with_rom(vec![]);
        let rom_data = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
        
        cpu.load_rom(rom_data.clone());
        
        // ROM이 메모리에 로드되었는지 확인
        for (i, &byte) in rom_data.iter().enumerate() {
            assert_eq!(cpu.read_byte((mmu::ROM_BASE_ADDR + i) as u16), byte);
        }
    }
}