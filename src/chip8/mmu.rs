// 메모리 맵 :
// 0x000-0x1FF - 칩 8 인터프리터. 글꼴 집합은 0x050-0x0A0에 있습니다.
// 0x200-0xFFF - 프로그램 ROM 및 작업 RAM

pub const FONT_BASE_ADDR: usize = 0x050;
pub const ROM_BASE_ADDR: usize = 0x200;

// 4K 메모리
const RAM_SIZE: usize = 0x1000;

pub struct MMU {
    // 적재 대상 rom
    rom: Vec<u8>,
    // 메모리
    ram: [u8; RAM_SIZE],
}

impl MMU {
    pub fn new(rom: Vec<u8>) -> Self {
        let mut mmu = MMU {
            rom,
            ram: [0; RAM_SIZE], // 메모리 초기화 생성
        };
        mmu.reset(); // Chip8에 rom 구동을 위한 메모리 초기화
        mmu
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.rom = rom;
        self.reset();
    }

    // 1. 폰트 셋을 가져옴
    // 2. rom을 메모리에 적재
    pub fn reset(&mut self) {
        // 새로운 rom을 가져오기전 메모리 초기화
        self.ram = [0; RAM_SIZE];
        // 폰트 셋 가져오기
        for (i, b) in [
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ]
            .iter()
            .enumerate()
        {
            self.ram[FONT_BASE_ADDR + i] = *b;
        }

        //ROM을 메모리에 적재
        for (i, b) in self.rom.iter().enumerate() {
            self.ram[ROM_BASE_ADDR + i] = *b;
        }
    }

    pub fn read_byte(&mut self, addr: usize) -> u8 {
        self.ram[addr]
    }

    pub fn write_byte(&mut self, addr: usize, value: u8) {
        self.ram[addr] = value
    }

    pub fn read_word(&mut self, addr: usize) -> u16 {
        ((self.read_byte(addr) as u16) << 8) | (self.read_byte(addr + 1) as u16)
    }

    // 메모리 포인터
    pub fn get_ram_ptr(&self) -> *const u8 {
        self.ram.as_ptr()
    }
}