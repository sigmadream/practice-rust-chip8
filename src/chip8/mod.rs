mod cpu; // CPU 모듈
mod mmu; // 메모리 관리 모듈

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;

pub struct Interpreter {
    pub cpu: cpu::CPU,
    keypad: [bool; 16],
}

impl Interpreter {
    pub fn new(rom: Vec<u8>) -> Self {
        let mmu = mmu::MMU::new(rom);
        let cpu = cpu::CPU::new(mmu);

        Interpreter {
            cpu,
            keypad: [false; 16],
        }
    }

    pub fn update_keypad(&mut self, keypad: [bool; 16]) {
        self.keypad = keypad;
    }

    pub fn step(&mut self) {
        self.cpu.step(self.keypad);
    }

    pub fn should_redraw(&self) -> bool {
        self.cpu.should_redraw()
    }

    pub fn should_beep(&self) -> bool {
        self.cpu.should_beep()
    }

    pub fn update_timers(&mut self) {
        self.cpu.update_timers();
    }

    pub fn get_vram(&self) -> [u8; HEIGHT * WIDTH] {
        self.cpu.vram
    }
}
