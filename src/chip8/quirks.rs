// CHIP-8은 구현마다 일부 명령어의 동작이 다릅니다.
// 참고: https://github.com/Timendus/chip8-test-suite (5-quirks)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quirks {
    // 8XY1/8XY2/8XY3 실행 후 VF를 0으로 초기화
    pub vf_reset: bool,
    // FX55/FX65 실행 후 I를 X+1 만큼 증가
    pub memory_increment_i: bool,
    // DXYN 실행 후 다음 프레임(vblank)까지 대기
    pub display_wait: bool,
    // 화면 밖으로 나간 스프라이트를 자름 (false면 반대편으로 넘어감)
    pub clipping: bool,
    // 8XY6/8XYE가 VY를 시프트하여 VX에 저장 (false면 VX를 직접 시프트)
    pub shift_uses_vy: bool,
    // BNNN을 BXNN으로 해석하여 V0 대신 VX를 더함
    pub jump_uses_vx: bool,
}

impl Quirks {
    // 원본 COSMAC VIP CHIP-8
    pub const fn chip8() -> Self {
        Quirks {
            vf_reset: true,
            memory_increment_i: true,
            display_wait: true,
            clipping: true,
            shift_uses_vy: true,
            jump_uses_vx: false,
        }
    }

    // SUPER-CHIP (HP48, modern) 동작
    pub const fn schip() -> Self {
        Quirks {
            vf_reset: false,
            memory_increment_i: false,
            display_wait: false,
            clipping: true,
            shift_uses_vy: false,
            jump_uses_vx: true,
        }
    }
}

impl Default for Quirks {
    fn default() -> Self {
        Self::chip8()
    }
}
