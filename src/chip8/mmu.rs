// 메모리 맵 :
// 0x000-0x1FF - 칩 8 인터프리터. 글꼴 집합은 0x050-0x09F에 있습니다.
// 0x200-0xFFF - 프로그램 ROM 및 작업 RAM

use super::Chip8Error;

pub const FONT_BASE_ADDR: u16 = 0x050;
pub const ROM_BASE_ADDR: u16 = 0x200;

// 4K 메모리
pub const RAM_SIZE: usize = 0x1000;
// 주소 공간은 12비트
pub const ADDR_MASK: u16 = 0x0FFF;
// 적재 가능한 최대 ROM 크기 (0x200-0xFFF)
pub const MAX_ROM_SIZE: usize = RAM_SIZE - ROM_BASE_ADDR as usize;

// 폰트 셋 (0-F, 문자당 5바이트)
const FONT_SET: [u8; 80] = [
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
];

pub struct Mmu {
    // reset 시 다시 적재하기 위해 보관하는 rom
    rom: Vec<u8>,
    // 메모리
    ram: [u8; RAM_SIZE],
}

impl Mmu {
    pub fn new(rom: Vec<u8>) -> Result<Self, Chip8Error> {
        Self::check_rom_size(&rom)?;
        let mut mmu = Mmu {
            rom,
            ram: [0; RAM_SIZE],
        };
        mmu.reset(); // Chip8에 rom 구동을 위한 메모리 초기화
        Ok(mmu)
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) -> Result<(), Chip8Error> {
        Self::check_rom_size(&rom)?;
        self.rom = rom;
        self.reset();
        Ok(())
    }

    fn check_rom_size(rom: &[u8]) -> Result<(), Chip8Error> {
        if rom.len() > MAX_ROM_SIZE {
            return Err(Chip8Error::RomTooLarge {
                size: rom.len(),
                max: MAX_ROM_SIZE,
            });
        }
        Ok(())
    }

    // 1. 메모리 초기화
    // 2. 폰트 셋 적재
    // 3. rom을 메모리에 적재
    pub fn reset(&mut self) {
        self.ram = [0; RAM_SIZE];

        let font = FONT_BASE_ADDR as usize;
        self.ram[font..font + FONT_SET.len()].copy_from_slice(&FONT_SET);

        let base = ROM_BASE_ADDR as usize;
        self.ram[base..base + self.rom.len()].copy_from_slice(&self.rom);
    }

    // 주소는 12비트로 감싸므로 범위를 벗어나도 panic 하지 않음
    pub fn read_byte(&self, addr: u16) -> u8 {
        self.ram[(addr & ADDR_MASK) as usize]
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        self.ram[(addr & ADDR_MASK) as usize] = value;
    }

    pub fn read_word(&self, addr: u16) -> u16 {
        u16::from_be_bytes([self.read_byte(addr), self.read_byte(addr.wrapping_add(1))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mmu_with(rom: Vec<u8>) -> Mmu {
        Mmu::new(rom).unwrap()
    }

    #[test]
    fn test_mmu_new() {
        let rom = vec![0x12, 0x34, 0x56, 0x78];
        let mmu = mmu_with(rom.clone());

        assert_eq!(mmu.rom, rom);
        assert_eq!(mmu.read_byte(FONT_BASE_ADDR), 0xF0);
        assert_eq!(mmu.read_byte(ROM_BASE_ADDR), 0x12);
    }

    #[test]
    fn test_load_rom() {
        let mut mmu = mmu_with(vec![0x11, 0x22]);

        mmu.load_rom(vec![0xAA, 0xBB, 0xCC]).unwrap();

        assert_eq!(mmu.read_byte(ROM_BASE_ADDR), 0xAA);
        assert_eq!(mmu.read_byte(ROM_BASE_ADDR + 1), 0xBB);
        assert_eq!(mmu.read_byte(ROM_BASE_ADDR + 2), 0xCC);
    }

    #[test]
    fn test_reset() {
        let mut mmu = mmu_with(vec![0x12, 0x34]);

        mmu.write_byte(0x100, 0xFF);
        mmu.write_byte(0x200, 0xEE);
        mmu.reset();

        assert_eq!(mmu.read_byte(0x100), 0x00);
        assert_eq!(mmu.read_byte(0x200), 0x12); // ROM 데이터는 다시 로드됨
    }

    #[test]
    fn test_font_loading() {
        let mmu = mmu_with(vec![]);

        // 0
        assert_eq!(mmu.read_byte(FONT_BASE_ADDR), 0xF0);
        assert_eq!(mmu.read_byte(FONT_BASE_ADDR + 4), 0xF0);
        // 1
        assert_eq!(mmu.read_byte(FONT_BASE_ADDR + 5), 0x20);
        assert_eq!(mmu.read_byte(FONT_BASE_ADDR + 9), 0x70);
        // A
        assert_eq!(mmu.read_byte(FONT_BASE_ADDR + 50), 0xF0);
        assert_eq!(mmu.read_byte(FONT_BASE_ADDR + 54), 0x90);
        // F 마지막 바이트
        assert_eq!(mmu.read_byte(FONT_BASE_ADDR + 79), 0x80);
    }

    #[test]
    fn test_read_write_byte() {
        let mut mmu = mmu_with(vec![]);

        mmu.write_byte(0x300, 0xAB);
        mmu.write_byte(0x500, 0xCD);

        assert_eq!(mmu.read_byte(0x300), 0xAB);
        assert_eq!(mmu.read_byte(0x500), 0xCD);
    }

    #[test]
    fn test_read_word() {
        let mut mmu = mmu_with(vec![]);

        mmu.write_byte(0x400, 0x12);
        mmu.write_byte(0x401, 0x34);

        assert_eq!(mmu.read_word(0x400), 0x1234);
    }

    #[test]
    fn test_address_wraps_at_4k() {
        let mut mmu = mmu_with(vec![]);

        // 0x1000은 0x000으로 감싸짐
        mmu.write_byte(0x1000, 0xAA);
        assert_eq!(mmu.read_byte(0x000), 0xAA);

        // 0xFFF에서 워드를 읽으면 두 번째 바이트는 0x000
        mmu.write_byte(0xFFF, 0xBB);
        assert_eq!(mmu.read_word(0xFFF), 0xBBAA);
    }

    #[test]
    fn test_empty_rom() {
        let mmu = mmu_with(vec![]);

        assert_eq!(mmu.read_byte(FONT_BASE_ADDR), 0xF0);
        assert_eq!(mmu.read_byte(ROM_BASE_ADDR), 0x00);
    }

    #[test]
    fn test_max_rom() {
        let rom = vec![0xAB; MAX_ROM_SIZE];
        let mmu = mmu_with(rom);

        assert_eq!(mmu.read_byte(0xFFF), 0xAB);
    }

    #[test]
    fn test_rom_too_large() {
        let rom = vec![0; MAX_ROM_SIZE + 1];

        assert!(matches!(
            Mmu::new(rom),
            Err(Chip8Error::RomTooLarge { size, max }) if size == MAX_ROM_SIZE + 1 && max == MAX_ROM_SIZE
        ));
    }
}
