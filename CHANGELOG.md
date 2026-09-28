# Changelog

## Fixed

- 명세와 다르게 동작하던 명령어를 수정 (`src/chip8/cpu.rs`)

| 명령어 | 이전 동작 | 수정 후 |
|---|---|---|
| 8XY5 | Vx == Vy일 때 VF=0 | 빌림이 없으므로 VF=1 (`>=` 비교) |
| 8XY4/5/6/7/E | VF를 먼저 쓰고 결과를 나중에 씀. X가 F이면 플래그가 사라짐 | 결과를 먼저 쓰고 VF를 마지막에 씀 |
| 0x0 계열 | 하위 4비트만 보고 해석해서 `0NN0`을 모두 화면 지우기로 처리하고, 그 외 `0NNN`은 panic | `00E0`, `00EE`만 정확히 일치할 때 실행하고, 나머지 `0NNN`은 무시 |
| FX1E | VF를 변경(Amiga 전용 동작). I가 12비트를 넘어 계속 커짐 | VF를 바꾸지 않음. I를 `& 0xFFF`로 감쌈 |
| DXYN | 모든 픽셀을 반대편으로 넘김 | 시작 좌표만 화면 크기로 감싸고, 화면 밖 픽셀은 잘라냄 (clipping quirk) |
| FX0A | 키를 누르는 즉시 완료 | 키를 눌렀다가 뗄 때 완료 |
| 9XY0 | 하위 니블을 검사하지 않음 | 하위 니블이 0일 때만 실행 |

- 잘못된 ROM으로 panic이 나던 경로를 막았음
  - 17단계 이상 서브루틴 호출 시 index out of bounds가 나던 문제: `Chip8Error::StackOverflow` 반환
  - 빈 스택에서 `00EE` 실행 시 뺄셈 overflow가 나던 문제: `Chip8Error::StackUnderflow` 반환
  - 지원하지 않는 opcode에서 `panic!`하던 문제: `Chip8Error::UnknownOpcode` 반환
  - 3584바이트보다 큰 ROM: `Chip8Error::RomTooLarge` 반환
  - EX9E/EXA1/FX29에서 Vx가 15보다 크면 panic하던 문제: 하위 4비트만 사용
  - I+n 주소 접근(DXYN, FX33, FX55, FX65)이 4KB를 넘으면 panic하던 문제: 모든 메모리 접근을 12비트로 감쌈

- 실행 루프의 타이밍을 수정 (`src/main.rs`)
  - minifb 기본 프레임 제한(4ms, 약 250fps) 때문에 타이머가 약 250Hz로 줄던 문제를 고침. `set_target_fps(60)`으로 60Hz에 맞춤
  - 실행 속도가 초당 약 1250 명령어였던 것을 초당 약 660 명령어(기본 `--speed 11`)로 조정

## Added

- quirk 설정 (`src/chip8/quirks.rs`)
  - `Quirks` 구조체: `vf_reset`, `memory_increment_i`, `display_wait`, `clipping`, `shift_uses_vy`, `jump_uses_vx`
  - 프리셋: `Quirks::chip8()`(원본 COSMAC VIP, 기본값), `Quirks::schip()`(SUPER-CHIP)
  - `Interpreter::with_quirks(rom, quirks)`
  - CLI 옵션 `--platform chip8|schip`
- 에러 타입 `Chip8Error` (`src/chip8/error.rs`). `Display`, `std::error::Error` 구현
- `Interpreter::clear_redraw()`: 화면을 그린 뒤 redraw 플래그를 내림
- `Cpu::seed_rng(seed)`: CXNN 결과를 재현할 수 있도록 난수 시드 고정
- `examples/headless.rs`: 창 없이 ROM을 실행하고 화면을 텍스트로 출력
- 단위 테스트 추가: VF가 결과 레지스터인 경우의 플래그, 8XY5/8XY7 경계값, quirk별 동작, 시작 좌표 감싸기와 clipping, display wait, 스택 에러, 알 수 없는 opcode, FX0A 키 떼기, 4KB 주소 감싸기, ROM 크기 제한
- README: 사용법, 키 매핑, quirk 표, 테스트 ROM 받는 방법과 실행 방법

## Changed

- API가 변경

| 이전 | 이후 |
|---|---|
| `Interpreter::new(rom) -> Interpreter` | `Interpreter::new(rom) -> Result<Interpreter, Chip8Error>` |
| `Interpreter::step()` | `Interpreter::step() -> Result<(), Chip8Error>` |
| `Interpreter::get_vram() -> [u8; N]` (복사) | `Interpreter::get_vram() -> &[u8; N]` (참조) |
| `CPU::step(keypad)` | `Cpu::step()` (키 입력은 `update_keypad_state`로 전달) |
| `CPU`, `MMU` | `Cpu`, `Mmu` |
| `Registers::pc`, `Registers::i`: `usize` | `u16` |
| `MMU::read_byte(&mut self, usize)` | `Mmu::read_byte(&self, u16)` |
| `MMU::new(rom) -> MMU`, `load_rom(rom)` | `Result`을 반환 |

- 동작이 변경
  - 기본 quirk가 원본 COSMAC VIP 동작. 시프트는 VY 기준이고, 8XY1/2/3 뒤에 VF를 초기화하며, FX55/FX65 뒤에 I가 증가하고, DXYN 뒤에 vblank를 기다림. 이전 동작에 가까운 게임은 `--platform schip`으로 실행
  - `should_redraw()`는 이제 매 `step()`마다 초기화되지 않고, `clear_redraw()`를 호출할 때까지 유지
  - `--speed`는 60Hz 한 프레임당 실행할 명령어 수. 기본값이 5에서 11로 바뀌었고 타입은 `u32`

- 내부 구조를 정리
  - 명령어 해석을 `(opcode >> 12, x, y, n)` 튜플 매칭으로 바꾸고, fetch 직후 pc를 2 증가시키는 방식으로 통일
  - 키패드 상태가 `Interpreter`, `CPU`, `step()` 인자 세 곳에 있던 것을 `Cpu` 한 곳으로 합침
  - 키 입력은 명령어마다가 아니라 프레임마다 한 번 읽음
  - 화면을 그릴 때 VRAM을 픽셀마다 2KB씩 복사하던 것(프레임당 약 4MB)을 참조 한 번으로 바꿈
  - ROM 읽기, 창 생성 실패 시 `unwrap` 대신 에러 메시지를 출력하고 종료
  - 오디오 장치가 없으면 경고만 출력하고 소리 없이 실행
  - 폰트 셋을 상수로 분리하고 `copy_from_slice`로 적재
  - `ThreadRng` 대신 시드를 지정할 수 있는 `StdRng` 사용
  - 중복된 `WIDTH`/`HEIGHT` 상수와 불필요한 `return`을 정리. clippy 경고 14건이 0건이 됨
- `cargo fmt` 적용

## Removed

- `MMU::get_ram_ptr()`: 사용처가 없는 raw pointer 반환 함수
- `Interpreter`의 중복 키패드 필드
