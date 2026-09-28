pub mod cpu; // CPU 모듈
mod error;
pub mod mmu; // 메모리 관리 모듈
mod quirks;

pub use error::Chip8Error;
pub use quirks::Quirks;

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;

pub struct Interpreter {
    pub cpu: cpu::Cpu,
}

impl Interpreter {
    // 원본 COSMAC VIP 동작(Quirks::chip8)으로 생성
    pub fn new(rom: Vec<u8>) -> Result<Self, Chip8Error> {
        Self::with_quirks(rom, Quirks::chip8())
    }

    pub fn with_quirks(rom: Vec<u8>, quirks: Quirks) -> Result<Self, Chip8Error> {
        let mmu = mmu::Mmu::new(rom)?;
        let cpu = cpu::Cpu::new(mmu, quirks);
        Ok(Interpreter { cpu })
    }

    pub fn update_keypad(&mut self, keypad: [bool; 16]) {
        self.cpu.update_keypad_state(keypad);
    }

    pub fn step(&mut self) -> Result<(), Chip8Error> {
        self.cpu.step()
    }

    pub fn should_redraw(&self) -> bool {
        self.cpu.should_redraw()
    }

    // 화면을 그린 뒤 호출하여 redraw 플래그를 내림
    pub fn clear_redraw(&mut self) {
        self.cpu.clear_redraw();
    }

    pub fn should_beep(&self) -> bool {
        self.cpu.should_beep()
    }

    // 60Hz 마다 호출
    pub fn update_timers(&mut self) {
        self.cpu.update_timers();
    }

    pub fn get_vram(&self) -> &[u8; HEIGHT * WIDTH] {
        &self.cpu.vram
    }
}
