use rand::{Rng, SeedableRng, rngs::StdRng};

use super::mmu::{self, ADDR_MASK, Mmu};
use super::{Chip8Error, HEIGHT, Quirks, WIDTH};

// 서브루틴 중첩 최대 깊이
const STACK_SIZE: usize = 16;

// 레지스터 구현
#[derive(Default)]
pub struct Registers {
    // 참고: VF 레지스터는 일부 명령어의 플래그 역할을 하므로 사용하지 않는 것이 좋습니다.
    // 덧셈 연산에서는 VF가 캐리 플래그이고, 뺄셈에서는 "빌림 없음" 플래그입니다.
    // 그리기 명령어에서는 픽셀 충돌 시 VF가 설정됩니다.
    pub v: [u8; 16],
    // 인덱스 레지스터 (12비트)
    pub i: u16,
    // 프로그램 카운터 (12비트)
    pub pc: u16,
    // 스택 포인터
    pub sp: usize,
    // 이 타이머는 게임 이벤트의 타이밍을 측정하는 데 사용됩니다.
    // 값을 설정하고 읽을 수 있습니다.
    pub delay: u8,
    // 이 타이머는 음향 효과에 사용됩니다.
    // 값이 0이 아니면 삐 소리가 납니다.
    pub sound: u8,
}

// FX0A는 키를 눌렀다가 뗄 때 완료됩니다.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum KeyWait {
    #[default]
    Idle,
    // 키가 눌리기를 기다리는 중
    Press,
    // 눌린 키가 떼어지기를 기다리는 중
    Release(u8),
}

// 키패드 구현
#[derive(Default)]
struct Keypad {
    state: [bool; 16],
    wait: KeyWait,
}

// CPU 구현
pub struct Cpu {
    pub mmu: Mmu,

    pub vram: [u8; HEIGHT * WIDTH],
    vram_changed: bool,

    pub registers: Registers,
    // 스택은 서브루틴이 호출될 때 반환 주소를 저장하는 데만 사용됩니다.
    stack: [u16; STACK_SIZE],
    keypad: Keypad,

    quirks: Quirks,
    // display wait quirk: DXYN 실행 후 다음 update_timers 호출까지 실행을 멈춤
    waiting_vblank: bool,

    rng: StdRng,
}

impl Cpu {
    pub fn new(mmu: Mmu, quirks: Quirks) -> Self {
        let mut cpu = Cpu {
            mmu,
            vram: [0; HEIGHT * WIDTH],
            vram_changed: false,
            registers: Registers::default(),
            stack: [0; STACK_SIZE],
            keypad: Keypad::default(),
            quirks,
            waiting_vblank: false,
            rng: StdRng::from_os_rng(),
        };

        cpu.reset_state();
        cpu
    }

    // 테스트 등에서 CXNN 결과를 재현할 수 있도록 난수 시드를 고정
    pub fn seed_rng(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    pub fn quirks(&self) -> Quirks {
        self.quirks
    }

    pub fn fetch_instruction(&self) -> u16 {
        self.mmu.read_word(self.registers.pc)
    }

    pub fn step(&mut self) -> Result<(), Chip8Error> {
        if self.waiting_vblank {
            return Ok(());
        }

        let opcode = self.fetch_instruction();
        self.registers.pc = self.registers.pc.wrapping_add(2) & ADDR_MASK;
        self.execute(opcode)
    }

    // 60Hz 마다 호출
    pub fn update_timers(&mut self) {
        self.registers.delay = self.registers.delay.saturating_sub(1);
        self.registers.sound = self.registers.sound.saturating_sub(1);
        self.waiting_vblank = false;
    }

    pub fn should_redraw(&self) -> bool {
        self.vram_changed
    }

    pub fn clear_redraw(&mut self) {
        self.vram_changed = false;
    }

    pub fn should_beep(&self) -> bool {
        self.registers.sound > 0
    }

    pub fn reset(&mut self) {
        self.mmu.reset();
        self.reset_state();
    }

    fn reset_state(&mut self) {
        self.vram = [0; HEIGHT * WIDTH];
        self.vram_changed = false;
        self.registers = Registers {
            pc: mmu::ROM_BASE_ADDR,
            ..Registers::default()
        };
        self.stack = [0; STACK_SIZE];
        self.keypad = Keypad::default();
        self.waiting_vblank = false;
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        self.mmu.read_byte(addr)
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) -> Result<(), Chip8Error> {
        self.mmu.load_rom(rom)?;
        self.reset_state();
        Ok(())
    }

    pub fn get_keypad_state(&self) -> &[bool; 16] {
        &self.keypad.state
    }

    pub fn is_keypad_waiting(&self) -> bool {
        self.keypad.wait != KeyWait::Idle
    }

    pub fn get_stack(&self) -> &[u16; STACK_SIZE] {
        &self.stack
    }

    pub fn update_keypad_state(&mut self, keypad: [bool; 16]) {
        self.keypad.state = keypad;
    }

    fn execute(&mut self, opcode: u16) -> Result<(), Chip8Error> {
        let x = ((opcode >> 8) & 0xF) as usize;
        let y = ((opcode >> 4) & 0xF) as usize;
        let n = (opcode & 0xF) as u8;
        let nn = (opcode & 0xFF) as u8;
        let nnn = opcode & 0x0FFF;

        match (opcode >> 12, x, y, n) {
            // disp_clear()
            (0x0, 0x0, 0xE, 0x0) => self.clear_screen(),
            // return
            (0x0, 0x0, 0xE, 0xE) => self.ret()?,
            // 0NNN: 기계어 루틴 호출. 인터프리터에서는 무시합니다.
            (0x0, ..) => {}
            // goto NNN
            (0x1, ..) => self.registers.pc = nnn,
            // *(0xNNN)()
            (0x2, ..) => self.call(nnn)?,
            // if (Vx == NN)
            (0x3, ..) => self.skip_if(self.registers.v[x] == nn),
            // if (Vx != NN)
            (0x4, ..) => self.skip_if(self.registers.v[x] != nn),
            // if (Vx == Vy)
            (0x5, _, _, 0x0) => self.skip_if(self.registers.v[x] == self.registers.v[y]),
            // Vx = NN
            (0x6, ..) => self.registers.v[x] = nn,
            // Vx += NN (VF는 변하지 않음)
            (0x7, ..) => self.registers.v[x] = self.registers.v[x].wrapping_add(nn),
            (0x8, ..) => self.op_8xy(opcode, x, y, n)?,
            // if (Vx != Vy)
            (0x9, _, _, 0x0) => self.skip_if(self.registers.v[x] != self.registers.v[y]),
            // I = NNN
            (0xA, ..) => self.registers.i = nnn,
            // PC = V0 + NNN (jump quirk: VX + NNN)
            (0xB, ..) => {
                let offset = if self.quirks.jump_uses_vx {
                    self.registers.v[x]
                } else {
                    self.registers.v[0]
                };
                self.registers.pc = (nnn + offset as u16) & ADDR_MASK;
            }
            // Vx = rand() & NN
            (0xC, ..) => self.registers.v[x] = self.rng.random::<u8>() & nn,
            // draw(Vx, Vy, N)
            (0xD, ..) => self.draw(x, y, n),
            // if (key() == Vx)
            (0xE, _, 0x9, 0xE) => self.skip_if(self.is_key_down(self.registers.v[x])),
            // if (key() != Vx)
            (0xE, _, 0xA, 0x1) => self.skip_if(!self.is_key_down(self.registers.v[x])),
            (0xF, ..) => self.op_fx(opcode, x, nn)?,
            _ => return Err(self.unknown_opcode(opcode)),
        }
        Ok(())
    }

    fn op_8xy(&mut self, opcode: u16, x: usize, y: usize, n: u8) -> Result<(), Chip8Error> {
        let vx = self.registers.v[x];
        let vy = self.registers.v[y];
        let reset_vf = self.quirks.vf_reset.then_some(0);
        let shift_src = if self.quirks.shift_uses_vy { vy } else { vx };

        let (result, flag) = match n {
            // Vx = Vy
            0x0 => (vy, None),
            // Vx = Vx | Vy
            0x1 => (vx | vy, reset_vf),
            // Vx = Vx & Vy
            0x2 => (vx & vy, reset_vf),
            // Vx = Vx ^ Vy
            0x3 => (vx ^ vy, reset_vf),
            // Vx = Vx + Vy, VF = 캐리
            0x4 => {
                let (r, carry) = vx.overflowing_add(vy);
                (r, Some(u8::from(carry)))
            }
            // Vx = Vx - Vy, VF = 빌림 없음
            0x5 => {
                let (r, borrow) = vx.overflowing_sub(vy);
                (r, Some(u8::from(!borrow)))
            }
            // Vx = Vx >> 1, VF = 밀려난 비트
            0x6 => (shift_src >> 1, Some(shift_src & 1)),
            // Vx = Vy - Vx, VF = 빌림 없음
            0x7 => {
                let (r, borrow) = vy.overflowing_sub(vx);
                (r, Some(u8::from(!borrow)))
            }
            // Vx = Vx << 1, VF = 밀려난 비트
            0xE => (shift_src << 1, Some(shift_src >> 7)),
            _ => return Err(self.unknown_opcode(opcode)),
        };

        // X가 F인 경우 플래그가 남도록 결과를 먼저 쓰고 VF를 나중에 씁니다.
        self.registers.v[x] = result;
        if let Some(flag) = flag {
            self.registers.v[0xF] = flag;
        }
        Ok(())
    }

    fn op_fx(&mut self, opcode: u16, x: usize, nn: u8) -> Result<(), Chip8Error> {
        match nn {
            // Vx = get_delay()
            0x07 => self.registers.v[x] = self.registers.delay,
            // Vx = get_key()
            0x0A => self.wait_for_key(x),
            // delay_timer(Vx)
            0x15 => self.registers.delay = self.registers.v[x],
            // sound_timer(Vx)
            0x18 => self.registers.sound = self.registers.v[x],
            // I += Vx (VF는 변하지 않음)
            0x1E => {
                self.registers.i = (self.registers.i + self.registers.v[x] as u16) & ADDR_MASK;
            }
            // I = sprite_addr[Vx]
            0x29 => {
                self.registers.i = mmu::FONT_BASE_ADDR + (self.registers.v[x] & 0xF) as u16 * 5;
            }
            // set_BCD(Vx)
            // *(I+0) = BCD(3)
            // *(I+1) = BCD(2)
            // *(I+2) = BCD(1)
            0x33 => {
                let val = self.registers.v[x];
                let i = self.registers.i;
                self.mmu.write_byte(i, val / 100);
                self.mmu.write_byte(i.wrapping_add(1), (val / 10) % 10);
                self.mmu.write_byte(i.wrapping_add(2), val % 10);
            }
            // reg_dump(Vx, &I)
            0x55 => {
                for r in 0..=x {
                    let addr = self.registers.i.wrapping_add(r as u16);
                    self.mmu.write_byte(addr, self.registers.v[r]);
                }
                self.increment_i_after_memory_op(x);
            }
            // reg_load(Vx, &I)
            0x65 => {
                for r in 0..=x {
                    let addr = self.registers.i.wrapping_add(r as u16);
                    self.registers.v[r] = self.mmu.read_byte(addr);
                }
                self.increment_i_after_memory_op(x);
            }
            _ => return Err(self.unknown_opcode(opcode)),
        }
        Ok(())
    }

    fn clear_screen(&mut self) {
        self.vram = [0; HEIGHT * WIDTH];
        self.vram_changed = true;
    }

    fn call(&mut self, addr: u16) -> Result<(), Chip8Error> {
        if self.registers.sp >= STACK_SIZE {
            return Err(Chip8Error::StackOverflow {
                pc: self.instruction_pc(),
            });
        }
        // pc는 이미 다음 명령어를 가리킴
        self.stack[self.registers.sp] = self.registers.pc;
        self.registers.sp += 1;
        self.registers.pc = addr;
        Ok(())
    }

    fn ret(&mut self) -> Result<(), Chip8Error> {
        if self.registers.sp == 0 {
            return Err(Chip8Error::StackUnderflow {
                pc: self.instruction_pc(),
            });
        }
        self.registers.sp -= 1;
        self.registers.pc = self.stack[self.registers.sp];
        Ok(())
    }

    fn skip_if(&mut self, condition: bool) {
        if condition {
            self.registers.pc = self.registers.pc.wrapping_add(2) & ADDR_MASK;
        }
    }

    fn is_key_down(&self, key: u8) -> bool {
        self.keypad.state[(key & 0xF) as usize]
    }

    // 시작 좌표는 화면 크기로 감싸고, 화면 밖으로 나가는 픽셀은
    // clipping quirk에 따라 자르거나 반대편으로 넘깁니다.
    fn draw(&mut self, x: usize, y: usize, height: u8) {
        let x0 = self.registers.v[x] as usize % WIDTH;
        let y0 = self.registers.v[y] as usize % HEIGHT;
        let clipping = self.quirks.clipping;

        self.registers.v[0xF] = 0;

        for row in 0..height as usize {
            let py = y0 + row;
            if clipping && py >= HEIGHT {
                break;
            }
            let sprite = self
                .mmu
                .read_byte(self.registers.i.wrapping_add(row as u16));

            for col in 0..8 {
                let px = x0 + col;
                if clipping && px >= WIDTH {
                    break;
                }
                if sprite & (0x80 >> col) == 0 {
                    continue;
                }

                let idx = (px % WIDTH) + (py % HEIGHT) * WIDTH;
                if self.vram[idx] == 1 {
                    self.registers.v[0xF] = 1;
                }
                self.vram[idx] ^= 1;
            }
        }

        self.vram_changed = true;
        if self.quirks.display_wait {
            self.waiting_vblank = true;
        }
    }

    // 키가 눌렸다가 떼어질 때까지 같은 명령어를 반복 실행합니다.
    fn wait_for_key(&mut self, x: usize) {
        match self.keypad.wait {
            KeyWait::Idle | KeyWait::Press => {
                self.keypad.wait = match self.keypad.state.iter().position(|&down| down) {
                    Some(key) => KeyWait::Release(key as u8),
                    None => KeyWait::Press,
                };
                self.repeat_instruction();
            }
            KeyWait::Release(key) => {
                if self.keypad.state[key as usize] {
                    self.repeat_instruction();
                } else {
                    self.registers.v[x] = key;
                    self.keypad.wait = KeyWait::Idle;
                }
            }
        }
    }

    fn repeat_instruction(&mut self) {
        self.registers.pc = self.instruction_pc();
    }

    fn increment_i_after_memory_op(&mut self, x: usize) {
        if self.quirks.memory_increment_i {
            self.registers.i = (self.registers.i + x as u16 + 1) & ADDR_MASK;
        }
    }

    // 현재 실행 중인 명령어의 주소 (step에서 pc를 미리 2 증가시킴)
    fn instruction_pc(&self) -> u16 {
        self.registers.pc.wrapping_sub(2) & ADDR_MASK
    }

    fn unknown_opcode(&self, opcode: u16) -> Chip8Error {
        Chip8Error::UnknownOpcode {
            opcode,
            pc: self.instruction_pc(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_KEYS: [bool; 16] = [false; 16];

    // 0x200부터 명령어들을 적재한 CPU
    fn cpu_with(program: &[u16], quirks: Quirks) -> Cpu {
        let rom = program.iter().flat_map(|op| op.to_be_bytes()).collect();
        let mut cpu = Cpu::new(Mmu::new(rom).unwrap(), quirks);
        cpu.seed_rng(0);
        cpu
    }

    fn chip8(program: &[u16]) -> Cpu {
        cpu_with(program, Quirks::chip8())
    }

    fn schip(program: &[u16]) -> Cpu {
        cpu_with(program, Quirks::schip())
    }

    fn run(cpu: &mut Cpu, steps: usize) {
        for _ in 0..steps {
            cpu.step().unwrap();
        }
    }

    fn keys(pressed: &[usize]) -> [bool; 16] {
        let mut state = NO_KEYS;
        for &k in pressed {
            state[k] = true;
        }
        state
    }

    #[test]
    fn test_cpu_initialization() {
        let cpu = chip8(&[]);

        assert_eq!(cpu.registers.pc, mmu::ROM_BASE_ADDR);
        assert_eq!(cpu.registers.sp, 0);
        assert_eq!(cpu.registers.i, 0);
        assert_eq!(cpu.registers.delay, 0);
        assert_eq!(cpu.registers.sound, 0);
        assert!(cpu.vram.iter().all(|&p| p == 0));
    }

    #[test]
    fn test_clear_display() {
        let mut cpu = chip8(&[0x00E0]);
        cpu.vram[0] = 1;
        cpu.vram[100] = 1;

        run(&mut cpu, 1);

        assert!(cpu.should_redraw());
        assert!(cpu.vram.iter().all(|&p| p == 0));
    }

    #[test]
    fn test_sys_instruction_is_ignored() {
        // 0x0NN0 형태도 화면 지우기로 오해하지 않아야 함
        let mut cpu = chip8(&[0x0120, 0x0000]);
        cpu.vram[0] = 1;

        run(&mut cpu, 2);

        assert_eq!(cpu.registers.pc, 0x204);
        assert_eq!(cpu.vram[0], 1);
        assert!(!cpu.should_redraw());
    }

    #[test]
    fn test_call_and_return() {
        let mut cpu = chip8(&[0x2206, 0x0000, 0x0000, 0x00EE]);

        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x206);
        assert_eq!(cpu.registers.sp, 1);
        assert_eq!(cpu.stack[0], 0x202);

        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x202);
        assert_eq!(cpu.registers.sp, 0);
    }

    #[test]
    fn test_stack_overflow() {
        // 자기 자신을 계속 호출
        let mut cpu = chip8(&[0x2200]);

        run(&mut cpu, STACK_SIZE);

        assert_eq!(cpu.step(), Err(Chip8Error::StackOverflow { pc: 0x200 }));
    }

    #[test]
    fn test_stack_underflow() {
        let mut cpu = chip8(&[0x00EE]);

        assert_eq!(cpu.step(), Err(Chip8Error::StackUnderflow { pc: 0x200 }));
    }

    #[test]
    fn test_unknown_opcode() {
        let mut cpu = chip8(&[0x5121]);

        assert_eq!(
            cpu.step(),
            Err(Chip8Error::UnknownOpcode {
                opcode: 0x5121,
                pc: 0x200
            })
        );
    }

    #[test]
    fn test_goto() {
        let mut cpu = chip8(&[0x1ABC]);

        run(&mut cpu, 1);

        assert_eq!(cpu.registers.pc, 0xABC);
    }

    #[test]
    fn test_skips() {
        // 3XNN, 4XNN, 5XY0, 9XY0 모두 조건이 참이면 다음 명령어를 건너뜀
        let mut cpu = chip8(&[0x3042, 0x0000, 0x4043, 0x0000, 0x5010, 0x0000, 0x9020]);
        cpu.registers.v[0] = 0x42;
        cpu.registers.v[1] = 0x42;
        cpu.registers.v[2] = 0x43;

        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x204);
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x208);
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x20C);
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x210);
    }

    #[test]
    fn test_set_and_add_register() {
        let mut cpu = chip8(&[0x60FF, 0x7002]);
        cpu.registers.v[0xF] = 0x55;

        run(&mut cpu, 2);

        assert_eq!(cpu.registers.v[0], 0x01); // 0xFF + 2 = 0x101 -> 0x01
        assert_eq!(cpu.registers.v[0xF], 0x55); // 7XNN은 VF를 바꾸지 않음
    }

    #[test]
    fn test_logic_operations() {
        let mut cpu = schip(&[0x8120, 0x8121, 0x8122, 0x8123]);
        cpu.registers.v[1] = 0x0F;
        cpu.registers.v[2] = 0xF0;

        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0xF0);
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0xF0);
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0xF0);
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0x00);
    }

    #[test]
    fn test_vf_reset_quirk() {
        let mut cpu = chip8(&[0x8121]);
        cpu.registers.v[0xF] = 1;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[0xF], 0);

        let mut cpu = schip(&[0x8121]);
        cpu.registers.v[0xF] = 1;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[0xF], 1);
    }

    #[test]
    fn test_add_with_carry() {
        let mut cpu = chip8(&[0x8124, 0x8124]);
        cpu.registers.v[1] = 0x80;
        cpu.registers.v[2] = 0x80;

        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0x00);
        assert_eq!(cpu.registers.v[0xF], 1);

        cpu.registers.v[1] = 0x10;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0x90);
        assert_eq!(cpu.registers.v[0xF], 0);
    }

    #[test]
    fn test_sub_flags() {
        // 8XY5: Vx >= Vy 이면 빌림 없음(VF=1). 같은 값도 빌림이 없음
        let mut cpu = chip8(&[0x8125, 0x8125, 0x8127]);
        cpu.registers.v[1] = 0x40;
        cpu.registers.v[2] = 0x40;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0x00);
        assert_eq!(cpu.registers.v[0xF], 1);

        cpu.registers.v[1] = 0x10;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0xD0);
        assert_eq!(cpu.registers.v[0xF], 0);

        // 8XY7: Vy >= Vx 이면 빌림 없음
        cpu.registers.v[1] = 0x40;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0x00);
        assert_eq!(cpu.registers.v[0xF], 1);
    }

    #[test]
    fn test_flag_wins_when_vf_is_destination() {
        // X가 F이면 연산 결과가 아니라 플래그가 VF에 남아야 함
        let mut cpu = schip(&[0x8F14, 0x8F15, 0x8F06, 0x8F0E, 0x8F17]);

        cpu.registers.v[0xF] = 0xFF;
        cpu.registers.v[1] = 0x01;
        run(&mut cpu, 1); // 0xFF + 0x01 -> 캐리
        assert_eq!(cpu.registers.v[0xF], 1);

        cpu.registers.v[0xF] = 0x10;
        cpu.registers.v[1] = 0x20;
        run(&mut cpu, 1); // 0x10 - 0x20 -> 빌림
        assert_eq!(cpu.registers.v[0xF], 0);

        cpu.registers.v[0xF] = 0x03;
        run(&mut cpu, 1); // 0x03 >> 1 -> 밀려난 비트 1
        assert_eq!(cpu.registers.v[0xF], 1);

        cpu.registers.v[0xF] = 0x40;
        run(&mut cpu, 1); // 0x40 << 1 -> 밀려난 비트 0
        assert_eq!(cpu.registers.v[0xF], 0);

        cpu.registers.v[0xF] = 0x10;
        cpu.registers.v[1] = 0x20;
        run(&mut cpu, 1); // 0x20 - 0x10 -> 빌림 없음
        assert_eq!(cpu.registers.v[0xF], 1);
    }

    #[test]
    fn test_shift_quirk() {
        // COSMAC: VY를 시프트하여 VX에 저장
        let mut cpu = chip8(&[0x8126, 0x812E]);
        cpu.registers.v[1] = 0x00;
        cpu.registers.v[2] = 0x85;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0x42);
        assert_eq!(cpu.registers.v[0xF], 1);
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0x0A);
        assert_eq!(cpu.registers.v[0xF], 1);

        // SUPER-CHIP: VX를 직접 시프트
        let mut cpu = schip(&[0x8126, 0x812E]);
        cpu.registers.v[1] = 0x85;
        cpu.registers.v[2] = 0x00;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0x42);
        assert_eq!(cpu.registers.v[0xF], 1);
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[1], 0x84);
        assert_eq!(cpu.registers.v[0xF], 0);
    }

    #[test]
    fn test_set_index_register() {
        let mut cpu = chip8(&[0xAABC]);

        run(&mut cpu, 1);

        assert_eq!(cpu.registers.i, 0xABC);
    }

    #[test]
    fn test_jump_quirk() {
        let mut cpu = chip8(&[0xB300]);
        cpu.registers.v[0] = 0x20;
        cpu.registers.v[3] = 0x40;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x320); // V0 + NNN

        let mut cpu = schip(&[0xB300]);
        cpu.registers.v[0] = 0x20;
        cpu.registers.v[3] = 0x40;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x340); // V3 + NNN
    }

    #[test]
    fn test_random_is_masked() {
        let mut cpu = chip8(&[0xC00F; 32]);

        for _ in 0..32 {
            run(&mut cpu, 1);
            assert_eq!(cpu.registers.v[0] & 0xF0, 0);
        }
    }

    #[test]
    fn test_random_with_seed_is_reproducible() {
        let mut a = chip8(&[0xC0FF]);
        let mut b = chip8(&[0xC0FF]);

        run(&mut a, 1);
        run(&mut b, 1);

        assert_eq!(a.registers.v[0], b.registers.v[0]);
    }

    #[test]
    fn test_draw_and_collision() {
        let mut cpu = schip(&[0xD121, 0xD121]);
        cpu.registers.i = 0x300;
        cpu.mmu.write_byte(0x300, 0xFF);

        run(&mut cpu, 1);
        assert!(cpu.should_redraw());
        assert!(cpu.vram[..8].iter().all(|&p| p == 1));
        assert_eq!(cpu.registers.v[0xF], 0);

        // 같은 위치에 다시 그리면 픽셀이 지워지고 충돌 플래그가 설정됨
        run(&mut cpu, 1);
        assert!(cpu.vram[..8].iter().all(|&p| p == 0));
        assert_eq!(cpu.registers.v[0xF], 1);
    }

    #[test]
    fn test_draw_start_position_wraps() {
        // 시작 좌표 (64+2, 32+1)은 (2, 1)로 감싸짐
        let mut cpu = schip(&[0xD121]);
        cpu.registers.v[1] = (WIDTH + 2) as u8;
        cpu.registers.v[2] = (HEIGHT + 1) as u8;
        cpu.registers.i = 0x300;
        cpu.mmu.write_byte(0x300, 0x80);

        run(&mut cpu, 1);

        assert_eq!(cpu.vram[2 + WIDTH], 1);
    }

    #[test]
    fn test_draw_clipping_quirk() {
        // 오른쪽 아래 모서리에 2x2 스프라이트
        let setup = |cpu: &mut Cpu| {
            cpu.registers.v[1] = (WIDTH - 1) as u8;
            cpu.registers.v[2] = (HEIGHT - 1) as u8;
            cpu.registers.i = 0x300;
            cpu.mmu.write_byte(0x300, 0xC0);
            cpu.mmu.write_byte(0x301, 0xC0);
        };

        let clip = Quirks {
            clipping: true,
            ..Quirks::schip()
        };
        let mut cpu = cpu_with(&[0xD122], clip);
        setup(&mut cpu);
        run(&mut cpu, 1);
        assert_eq!(cpu.vram.iter().filter(|&&p| p == 1).count(), 1);
        assert_eq!(cpu.vram[WIDTH * HEIGHT - 1], 1);

        let wrap = Quirks {
            clipping: false,
            ..Quirks::schip()
        };
        let mut cpu = cpu_with(&[0xD122], wrap);
        setup(&mut cpu);
        run(&mut cpu, 1);
        assert_eq!(cpu.vram.iter().filter(|&&p| p == 1).count(), 4);
        assert_eq!(cpu.vram[0], 1); // (0, 0)으로 넘어감
    }

    #[test]
    fn test_display_wait_quirk() {
        let mut cpu = chip8(&[0xD001, 0x6005]);

        run(&mut cpu, 2); // 그리기 후 vblank까지 멈춤
        assert_eq!(cpu.registers.pc, 0x202);
        assert_eq!(cpu.registers.v[0], 0);

        cpu.update_timers();
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[0], 5);
    }

    #[test]
    fn test_key_skips() {
        let mut cpu = chip8(&[0xE09E, 0x0000, 0xE0A1]);
        cpu.registers.v[0] = 5;

        cpu.update_keypad_state(keys(&[5]));
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x204);

        cpu.update_keypad_state(NO_KEYS);
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x208);
    }

    #[test]
    fn test_key_skip_masks_register() {
        // Vx가 15보다 커도 panic 하지 않고 하위 4비트만 사용
        let mut cpu = chip8(&[0xE09E]);
        cpu.registers.v[0] = 0x15;
        cpu.update_keypad_state(keys(&[5]));

        run(&mut cpu, 1);

        assert_eq!(cpu.registers.pc, 0x204);
    }

    #[test]
    fn test_wait_for_key_press_and_release() {
        let mut cpu = chip8(&[0xF00A]);

        run(&mut cpu, 1);
        assert_eq!(cpu.registers.pc, 0x200);
        assert!(cpu.is_keypad_waiting());

        // 누르고 있는 동안은 계속 대기
        cpu.update_keypad_state(keys(&[7]));
        run(&mut cpu, 3);
        assert_eq!(cpu.registers.pc, 0x200);

        // 떼는 순간 완료
        cpu.update_keypad_state(NO_KEYS);
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[0], 7);
        assert_eq!(cpu.registers.pc, 0x202);
        assert!(!cpu.is_keypad_waiting());
    }

    #[test]
    fn test_timer_operations() {
        let mut cpu = chip8(&[0xF007, 0xF015, 0xF018]);
        cpu.registers.delay = 50;

        run(&mut cpu, 1);
        assert_eq!(cpu.registers.v[0], 50);

        cpu.registers.v[0] = 100;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.delay, 100);

        cpu.registers.v[0] = 75;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.sound, 75);
    }

    #[test]
    fn test_timer_decrement() {
        let mut cpu = chip8(&[]);
        cpu.registers.delay = 10;
        cpu.registers.sound = 5;

        for _ in 0..12 {
            cpu.update_timers();
        }

        assert_eq!(cpu.registers.delay, 0);
        assert_eq!(cpu.registers.sound, 0);
    }

    #[test]
    fn test_sound_timer() {
        let mut cpu = chip8(&[]);
        assert!(!cpu.should_beep());

        cpu.registers.sound = 1;
        assert!(cpu.should_beep());

        cpu.update_timers();
        assert!(!cpu.should_beep());
    }

    #[test]
    fn test_add_to_index_does_not_touch_vf() {
        let mut cpu = chip8(&[0xF01E, 0xF01E]);
        cpu.registers.i = 0x100;
        cpu.registers.v[0] = 0x50;

        run(&mut cpu, 1);
        assert_eq!(cpu.registers.i, 0x150);
        assert_eq!(cpu.registers.v[0xF], 0);

        // 12비트를 넘어가면 감싸짐
        cpu.registers.i = 0xFF0;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.i, 0x040);
        assert_eq!(cpu.registers.v[0xF], 0);
    }

    #[test]
    fn test_font_address() {
        let mut cpu = chip8(&[0xF029, 0xF029]);

        cpu.registers.v[0] = 5;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.i, mmu::FONT_BASE_ADDR + 25);

        // 하위 4비트만 사용
        cpu.registers.v[0] = 0x1A;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.i, mmu::FONT_BASE_ADDR + 50);
    }

    #[test]
    fn test_bcd_operation() {
        let mut cpu = chip8(&[0xF033]);
        cpu.registers.i = 0x300;
        cpu.registers.v[0] = 123;

        run(&mut cpu, 1);

        assert_eq!(cpu.read_byte(0x300), 1);
        assert_eq!(cpu.read_byte(0x301), 2);
        assert_eq!(cpu.read_byte(0x302), 3);
    }

    #[test]
    fn test_register_dump_and_load() {
        let mut cpu = schip(&[0xF455, 0xF465]);
        cpu.registers.i = 0x300;
        for r in 0..5 {
            cpu.registers.v[r] = r as u8 + 10;
        }

        run(&mut cpu, 1);
        for r in 0..5 {
            assert_eq!(cpu.read_byte(0x300 + r as u16), r as u8 + 10);
        }

        cpu.registers.v = [0; 16];
        run(&mut cpu, 1);
        for r in 0..5 {
            assert_eq!(cpu.registers.v[r], r as u8 + 10);
        }
        assert_eq!(cpu.registers.v[5], 0);
    }

    #[test]
    fn test_memory_quirk() {
        let mut cpu = chip8(&[0xF255]);
        cpu.registers.i = 0x300;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.i, 0x303);

        let mut cpu = schip(&[0xF255]);
        cpu.registers.i = 0x300;
        run(&mut cpu, 1);
        assert_eq!(cpu.registers.i, 0x300);
    }

    #[test]
    fn test_cpu_reset() {
        let mut cpu = chip8(&[0x1234]);
        cpu.registers.v[0] = 0xFF;
        cpu.registers.i = 0x0FF;
        cpu.registers.delay = 50;
        cpu.registers.sound = 30;
        cpu.vram[0] = 1;
        cpu.mmu.write_byte(0x200, 0x00);

        cpu.reset();

        assert_eq!(cpu.registers.pc, mmu::ROM_BASE_ADDR);
        assert_eq!(cpu.registers.i, 0);
        assert_eq!(cpu.registers.delay, 0);
        assert_eq!(cpu.registers.sound, 0);
        assert_eq!(cpu.registers.v[0], 0);
        assert!(cpu.vram.iter().all(|&p| p == 0));
        assert_eq!(cpu.fetch_instruction(), 0x1234);
    }

    #[test]
    fn test_rom_loading() {
        let mut cpu = chip8(&[]);
        let rom_data = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];

        cpu.load_rom(rom_data.clone()).unwrap();

        for (i, &byte) in rom_data.iter().enumerate() {
            assert_eq!(cpu.read_byte(mmu::ROM_BASE_ADDR + i as u16), byte);
        }
    }
}
