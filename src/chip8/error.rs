use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chip8Error {
    // ROM이 0x200-0xFFF 영역보다 큼
    RomTooLarge { size: usize, max: usize },
    // 서브루틴 호출이 16단계를 넘음
    StackOverflow { pc: u16 },
    // 빈 스택에서 00EE 실행
    StackUnderflow { pc: u16 },
    // 해석할 수 없는 명령어
    UnknownOpcode { opcode: u16, pc: u16 },
}

impl fmt::Display for Chip8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Chip8Error::RomTooLarge { size, max } => {
                write!(f, "ROM is too large: {size} bytes (max {max} bytes)")
            }
            Chip8Error::StackOverflow { pc } => write!(f, "stack overflow @ 0x{pc:03X}"),
            Chip8Error::StackUnderflow { pc } => write!(f, "stack underflow @ 0x{pc:03X}"),
            Chip8Error::UnknownOpcode { opcode, pc } => {
                write!(f, "unknown opcode 0x{opcode:04X} @ 0x{pc:03X}")
            }
        }
    }
}

impl std::error::Error for Chip8Error {}
