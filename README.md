# practice-rust-chip8

Rust(2024 edition)로 작성한 모듈형 CHIP-8 인터프리터 및 에뮬레이터입니다.
재사용 가능한 코어 라이브러리(libemulators)와 데스크톱 GUI 실행 바이너리(emulators)로 구성되어 있습니다.

## 주요 특징

- 정확한 CHIP-8 명세 구현: 표준 35개 명령어 세트와 타이머, 16진수 키패드 완벽 지원
- 듀얼 플랫폼 호환(Quirks): 원본 COSMAC VIP(chip8) 및 SUPER-CHIP modern(schip) 프리셋 제공
- 60Hz 디스플레이: minifb 기반 64x32 해상도(8배 확대, 512x256 윈도우) 렌더링
- 사운드 재생: rodio 기반 400Hz 신스음 비프 재생(오디오 장치 미지원 환경에서는 무음 안전 폴백)
- 안정적인 에러 처리: 스택 오버플로우/언더플로우, ROM 용량 초과(최대 3584바이트), 미정의 opcode를 panic 없이 Result 타입으로 처리
- 헤드리스 실행 모드: GUI 창 없이 터미널 아스키 아트로 렌더링 가능한 headless 예제 제공
- 높은 테스트 통과율: 62개 유닛 및 통합 테스트 통과, Timendus chip8-test-suite(corax+, flags, quirks) 검증 완료

## 프로젝트 구조

- src/lib.rs: 라이브러리 루트 모듈
- src/main.rs: CLI 파서(clap), 그래픽 창 및 오디오 루프 진입점
- src/chip8/mod.rs: Interpreter 메인 인터페이스 및 디스플레이 상수(WIDTH=64, HEIGHT=32)
- src/chip8/cpu.rs: CPU 레지스터(V0~VF, I, PC, SP, DT, ST), 스택, 키패드 상태, opcode 디코딩 및 실행
- src/chip8/mmu.rs: 4KB RAM, ROM 로드(0x200), 내장 폰트(0x050~0x09F)
- src/chip8/quirks.rs: 플랫폼별 비표준 동작 설정 플래그 및 프리셋
- src/chip8/error.rs: 인터프리터 에러 열거형(Chip8Error)
- examples/headless.rs: 터미널 텍스트 출력 기반 테스트 러너 예제
- tests/integration_tests.rs: 명령어 및 CPU 동작 통합 테스트
- roms/: 기본 제공 샘플 ROM 파일(IBMLOGO, PONG, TETRIS)

## 빌드 및 요구 사항

- Rust 1.85 이상 (2024 edition 지원)

빌드 명령어:

```sh
cargo build --release
```

컴파일된 실행 파일은 target/release/emulators 에 생성됩니다.

## 실행 방법

기본 실행:

```sh
cargo run --release -- roms/IBMLOGO
cargo run --release -- roms/PONG
cargo run --release -- roms/TETRIS --speed 15 --platform schip
```

직접 빌드된 바이너리 실행:

```sh
./target/release/emulators roms/PONG
```

### CLI 옵션

| 옵션 | 기본값 | 설명 |
|---|---|---|
| `<ROM>` | (필수) | 실행할 CHIP-8 ROM 파일 경로 |
| `--speed <SPEED>` | 11 | 60Hz 1프레임당 실행할 명령어 수 (11 = 초당 약 660 명령어) |
| `--platform <PLATFORM>` | chip8 | 호환성 프리셋. chip8(COSMAC VIP) 또는 schip(SUPER-CHIP) |
| `-h, --help` | - | 도움말 출력 |
| `-V, --version` | - | 버전 정보 출력 |

종료는 창에서 ESC 키를 누르거나 창 닫기 버튼을 누릅니다.

## 키 매핑

CHIP-8의 4x4 16진수 키패드는 현대 PC 키보드의 4x4 키 영역에 다음과 같이 대응됩니다.

```
CHIP-8 키패드          PC 키보드
+-+-+-+-+            +-+-+-+-+
|1|2|3|C|            |1|2|3|4|
+-+-+-+-+            +-+-+-+-+
|4|5|6|D|     ->     |Q|W|E|R|
+-+-+-+-+            +-+-+-+-+
|7|8|9|E|            |A|S|D|F|
+-+-+-+-+            +-+-+-+-+
|A|0|B|F|            |Z|X|C|V|
+-+-+-+-+            +-+-+-+-+
```

FX0A 명령어는 키를 눌렀다가 뗄 때 입력이 완료되는 동작을 충실히 구현하고 있습니다.

## Quirks 호환 모드

CHIP-8은 개발 시기와 기기마다 일부 명령어 동작 방식에 차이가 있습니다.
기본값은 원본 COSMAC VIP 동작(chip8)이며, 현대적인 게임이나 SUPER-CHIP용 게임이 이상 동작할 경우 `--platform schip`을 지정합니다.

| Quirk 설정 항목 | chip8 (기본값) | schip | 설명 |
|---|---|---|---|
| `vf_reset` | O | X | 8XY1, 8XY2, 8XY3(OR, AND, XOR) 연산 후 VF를 0으로 초기화 |
| `memory_increment_i` | O | X | FX55, FX65(메모리 덤프/로드) 실행 후 I 레지스터를 X + 1 만큼 증가 |
| `display_wait` | O | X | DXYN(화면 그리기) 실행 후 수직 동기화(vblank)까지 CPU 대기 |
| `clipping` | O | O | 화면 경계를 벗어나는 스프라이트를 반대편으로 넘기지 않고 잘라냄 |
| `shift_uses_vy` | O | X | 8XY6, 8XYE(시프트) 연산 시 VY를 읽어 시프트한 뒤 VX에 저장 (X면 VX 직접 시프트) |
| `jump_uses_vx` | X | O | BNNN 점프 시 V0 + NNN 대신 VX + NNN(BXNN)으로 분기 |

## 라이브러리 사용법 (libemulators)

이 프로젝트는 cdylib 및 rlib 형태의 libemulators 크레이트로 제공되므로, 다른 Rust 프로젝트에서 CHIP-8 코어를 직접 임베딩하여 사용할 수 있습니다.

```rust
use libemulators::chip8::{Interpreter, Quirks, WIDTH, HEIGHT};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rom_data = std::fs::read("roms/PONG")?;

    // Quirks 프리셋을 지정하여 인터프리터 생성
    let mut vm = Interpreter::with_quirks(rom_data, Quirks::chip8())?;

    // 16개 키패드 입력 상태 갱신
    let mut keypad = [false; 16];
    keypad[1] = true;
    vm.update_keypad(keypad);

    // 명령어 1단계 실행
    vm.step()?;

    // 60Hz 타이머 갱신 (Delay Timer, Sound Timer)
    vm.update_timers();

    // 화면 갱신 여부 확인 및 VRAM 읽기
    if vm.should_redraw() {
        let vram: &[u8; WIDTH * HEIGHT] = vm.get_vram();
        // 렌더링 버퍼 처리...
        vm.clear_redraw();
    }

    // 비프음 재생 여부 확인
    if vm.should_beep() {
        // 비프음 출력...
    }

    Ok(())
}
```

## 테스트 및 검증

### 기본 테스트

저장소에 포함된 모든 단위 테스트 및 통합 테스트를 실행합니다.

```sh
cargo test
```

### 테스트 ROM 검증

Timendus CHIP-8 Test Suite를 다운로드하여 호환성을 검증할 수 있습니다.
(GPL-3.0 라이선스 ROM이므로 저장소에 포함되지 않으며, test-roms 디렉터리는 커밋하지 마세요.)

다운로드 (Bash):

```sh
mkdir -p test-roms
for r in 1-chip8-logo 2-ibm-logo 3-corax+ 4-flags 5-quirks 6-keypad 7-beep; do
  curl -fL -o "test-roms/$r.ch8" "https://github.com/Timendus/chip8-test-suite/raw/main/bin/$r.ch8"
done
```

다운로드 (PowerShell):

```powershell
New-Item -ItemType Directory -Force -Path "test-roms"
$roms = @("1-chip8-logo", "2-ibm-logo", "3-corax+", "4-flags", "5-quirks", "6-keypad", "7-beep")
foreach ($r in $roms) {
    Invoke-WebRequest -Uri "https://github.com/Timendus/chip8-test-suite/raw/main/bin/$r.ch8" -OutFile "test-roms/$r.ch8"
}
```

헤드리스 모드로 자동 실행 검증:
(0x1FF 메모리 번지에 값을 미리 작성하여 테스트 ROM의 인터랙티브 메뉴를 스킵할 수 있습니다.)

```sh
# 사용법: cargo run --example headless -- <ROM> [프레임 수] [chip8|schip] [0x1FF 값]
cargo run --example headless -- test-roms/3-corax+.ch8 300
cargo run --example headless -- test-roms/4-flags.ch8 300
cargo run --example headless -- test-roms/5-quirks.ch8 900 chip8 1
cargo run --example headless -- test-roms/5-quirks.ch8 900 schip 2
```

대화형 테스트 ROM 실행:

```sh
# 키 입력 확인
cargo run --release -- test-roms/6-keypad.ch8

# 비프음 재생 확인
cargo run --release -- test-roms/7-beep.ch8
```

### 테스트 통과 현황

| 테스트 항목 | chip8 | schip | 결과 |
|---|---|---|---|
| 3-corax+ (기본 Opcode 검증) | 통과 | - | 전체 통과 |
| 4-flags (플래그 및 연산 레지스터) | 통과 | - | 전체 통과 |
| 5-quirks (Quirk별 분기/동작) | 통과 (1번 모드) | 통과 (2번 모드) | 전체 통과 |

## 내장 샘플 ROM

저장소의 roms 디렉터리에는 라이선스에 저촉되지 않는 기본 샘플 ROM이 포함되어 있습니다.

- roms/IBMLOGO: 고전 IBM 로고 디스플레이 데모
- roms/PONG: 퐁 2인용 탁구 게임
- roms/TETRIS: 테트리스 블록 게임 (--speed 15 --platform schip 권장)

## 참고 자료

- Guide to making a CHIP-8 emulator: https://tobiasvl.github.io/blog/write-a-chip-8-emulator/
- Timendus CHIP-8 Test Suite: https://github.com/Timendus/chip8-test-suite
- Cowgod's Chip-8 Technical Reference: http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
