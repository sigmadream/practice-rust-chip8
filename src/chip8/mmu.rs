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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmu_new() {
        let rom = vec![0x12, 0x34, 0x56, 0x78];
        let mmu = MMU::new(rom.clone());
        
        // ROM이 올바르게 저장되었는지 확인
        assert_eq!(mmu.rom, rom);
        
        // 메모리가 초기화되었는지 확인 (폰트와 ROM이 로드됨)
        assert_eq!(mmu.ram[FONT_BASE_ADDR], 0xF0); // 폰트 데이터 확인
        assert_eq!(mmu.ram[ROM_BASE_ADDR], 0x12); // ROM 데이터 확인
    }

    #[test]
    fn test_load_rom() {
        let initial_rom = vec![0x11, 0x22];
        let mut mmu = MMU::new(initial_rom);
        
        let new_rom = vec![0xAA, 0xBB, 0xCC];
        mmu.load_rom(new_rom.clone());
        
        // 새로운 ROM이 로드되었는지 확인
        assert_eq!(mmu.rom, new_rom);
        assert_eq!(mmu.ram[ROM_BASE_ADDR], 0xAA);
        assert_eq!(mmu.ram[ROM_BASE_ADDR + 1], 0xBB);
        assert_eq!(mmu.ram[ROM_BASE_ADDR + 2], 0xCC);
    }

    #[test]
    fn test_reset() {
        let rom = vec![0x12, 0x34];
        let mut mmu = MMU::new(rom);
        
        // 메모리에 다른 값들을 써넣음
        mmu.write_byte(0x100, 0xFF);
        mmu.write_byte(0x200, 0xEE);
        
        // 리셋 실행
        mmu.reset();
        
        // 메모리가 초기화되었는지 확인
        assert_eq!(mmu.ram[0x100], 0x00);
        assert_eq!(mmu.ram[0x200], 0x12); // ROM 데이터는 다시 로드됨
    }

    #[test]
    fn test_font_loading() {
        let rom = vec![0x12];
        let mmu = MMU::new(rom);
        
        // 폰트 데이터가 올바른 위치에 로드되었는지 확인
        assert_eq!(mmu.ram[FONT_BASE_ADDR], 0xF0);     // 0
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 1], 0x90); // 0
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 2], 0x90); // 0
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 3], 0x90); // 0
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 4], 0xF0); // 0
        
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 5], 0x20); // 1
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 6], 0x60); // 1
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 7], 0x20); // 1
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 8], 0x20); // 1
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 9], 0x70); // 1
        
        // A 문자 확인
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 50], 0xF0); // A
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 51], 0x90); // A
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 52], 0xF0); // A
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 53], 0x90); // A
        assert_eq!(mmu.ram[FONT_BASE_ADDR + 54], 0x90); // A
    }

    #[test]
    fn test_read_write_byte() {
        let rom = vec![0x12];
        let mut mmu = MMU::new(rom);
        
        // 바이트 쓰기 테스트
        mmu.write_byte(0x300, 0xAB);
        assert_eq!(mmu.read_byte(0x300), 0xAB);
        
        // 다른 주소에 쓰기
        mmu.write_byte(0x500, 0xCD);
        assert_eq!(mmu.read_byte(0x500), 0xCD);
        
        // 원래 값은 그대로 유지
        assert_eq!(mmu.read_byte(0x300), 0xAB);
    }

    #[test]
    fn test_read_word() {
        let rom = vec![0x12];
        let mut mmu = MMU::new(rom);
        
        // 워드 쓰기 (두 바이트)
        mmu.write_byte(0x400, 0x12);
        mmu.write_byte(0x401, 0x34);
        
        // 워드 읽기 테스트
        let word = mmu.read_word(0x400);
        assert_eq!(word, 0x1234);
        
        // 다른 워드 테스트
        mmu.write_byte(0x600, 0xAB);
        mmu.write_byte(0x601, 0xCD);
        let word2 = mmu.read_word(0x600);
        assert_eq!(word2, 0xABCD);
    }

    #[test]
    fn test_rom_loading() {
        let rom = vec![0x12, 0x34, 0x56, 0x78, 0x9A];
        let mmu = MMU::new(rom);
        
        // ROM이 올바른 위치에 로드되었는지 확인
        assert_eq!(mmu.ram[ROM_BASE_ADDR], 0x12);
        assert_eq!(mmu.ram[ROM_BASE_ADDR + 1], 0x34);
        assert_eq!(mmu.ram[ROM_BASE_ADDR + 2], 0x56);
        assert_eq!(mmu.ram[ROM_BASE_ADDR + 3], 0x78);
        assert_eq!(mmu.ram[ROM_BASE_ADDR + 4], 0x9A);
    }

    #[test]
    fn test_get_ram_ptr() {
        let rom = vec![0x12];
        let mmu = MMU::new(rom);
        
        let ptr = mmu.get_ram_ptr();
        
        // 포인터가 null이 아닌지 확인
        assert!(!ptr.is_null());
        
        // 포인터를 통해 메모리 접근 가능한지 확인
        unsafe {
            assert_eq!(*ptr.add(FONT_BASE_ADDR), 0xF0);
            assert_eq!(*ptr.add(ROM_BASE_ADDR), 0x12);
        }
    }

    #[test]
    fn test_memory_bounds() {
        let rom = vec![0x12];
        let mut mmu = MMU::new(rom);
        
        // 메모리 경계 테스트
        mmu.write_byte(0x000, 0xAA);
        mmu.write_byte(0xFFF, 0xBB);
        
        assert_eq!(mmu.read_byte(0x000), 0xAA);
        assert_eq!(mmu.read_byte(0xFFF), 0xBB);
    }

    #[test]
    fn test_word_alignment() {
        let rom = vec![0x12];
        let mut mmu = MMU::new(rom);
        
        // 워드 정렬 테스트
        mmu.write_byte(0x100, 0x12);
        mmu.write_byte(0x101, 0x34);
        mmu.write_byte(0x102, 0x56);
        mmu.write_byte(0x103, 0x78);
        
        assert_eq!(mmu.read_word(0x100), 0x1234);
        assert_eq!(mmu.read_word(0x102), 0x5678);
    }

    #[test]
    fn test_empty_rom() {
        let rom = vec![];
        let mmu = MMU::new(rom);
        
        // 빈 ROM으로 초기화해도 폰트는 로드되어야 함
        assert_eq!(mmu.ram[FONT_BASE_ADDR], 0xF0);
        
        // ROM 영역은 0으로 초기화되어야 함
        assert_eq!(mmu.ram[ROM_BASE_ADDR], 0x00);
    }

    #[test]
    fn test_large_rom() {
        let rom: Vec<u8> = (0..100).collect(); // 0부터 99까지의 값들
        let mmu = MMU::new(rom);
        
        // 큰 ROM이 올바르게 로드되었는지 확인
        for i in 0..100 {
            assert_eq!(mmu.ram[ROM_BASE_ADDR + i], i as u8);
        }
    }
}