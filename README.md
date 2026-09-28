# practice-rust-chip8

Rust로 작성한 CHIP-8 인터프리터입니다.

## 실행

```sh
cargo run --release -- roms/PONG
cargo run --release -- roms/TETRIS --speed 15 --platform schip
```

| 옵션 | 기본값 | 설명 |
|---|---|---|
| `--speed` | 11 | 60Hz 한 프레임당 실행할 명령어 수 (11 = 약 660 명령어/초) |
| `--platform` | chip8 | quirk 프리셋. `chip8`(원본 COSMAC VIP) 또는 `schip`(SUPER-CHIP) |

ESC로 종료합니다.

## 키 매핑

```
CHIP-8        키보드
1 2 3 C       1 2 3 4
4 5 6 D   ->  Q W E R
7 8 9 E       A S D F
A 0 B F       Z X C V
```

## Quirks

CHIP-8은 구현마다 일부 명령어의 동작이 다릅니다. 게임이 이상하게 동작하면 `--platform schip`을 시도해 보세요.

| quirk | chip8 | schip |
|---|---|---|
| 8XY1/2/3 실행 후 VF 초기화 | O | X |
| FX55/FX65 실행 후 I 증가 | O | X |
| DXYN 실행 후 vblank 대기 | O | X |
| 화면 밖 스프라이트 자르기 | O | O |
| 8XY6/8XYE가 VY를 시프트 | O | X |
| BNNN을 BXNN(VX + NNN)으로 해석 | X | O |

## 테스트

```sh
cargo test
```

### 테스트 ROM

[Timendus/chip8-test-suite](https://github.com/Timendus/chip8-test-suite)의 `bin/` 폴더에서 테스트 ROM을 받을 수 있습니다. GPL-3.0 라이선스이므로 저장소에는 포함하지 않습니다(`test-roms/`는 커밋하지 마세요).

```sh
mkdir -p test-roms
for r in 1-chip8-logo 2-ibm-logo 3-corax+ 4-flags 5-quirks 6-keypad 7-beep; do
  curl -fL -o "test-roms/$r.ch8" "https://github.com/Timendus/chip8-test-suite/raw/main/bin/$r.ch8"
done
```

`examples/headless.rs`로 창 없이 실행해 결과 화면을 텍스트로 확인할 수 있습니다. 테스트 ROM은 0x1FF에 값을 써 두면 메뉴 선택을 건너뜁니다.

```sh
# cargo run --example headless -- <ROM> [frames] [chip8|schip] [0x1FF 값]
cargo run --example headless -- test-roms/3-corax+.ch8 300
cargo run --example headless -- test-roms/4-flags.ch8 300
cargo run --example headless -- test-roms/5-quirks.ch8 900 chip8 1
cargo run --example headless -- test-roms/5-quirks.ch8 900 schip 2
```

키 입력이 필요한 `6-keypad`, 소리를 확인하는 `7-beep`은 창 모드로 실행합니다.

```sh
cargo run --release -- test-roms/6-keypad.ch8
```

| ROM | chip8 | schip |
|---|---|---|
| 3-corax+ | 전체 통과 | - |
| 4-flags | 전체 통과 | - |
| 5-quirks | 전체 통과 | 전체 통과 |

## 참고

- [Guide to making a CHIP-8 emulator](https://tobiasvl.github.io/blog/write-a-chip-8-emulator/)
- [Timendus/chip8-test-suite](https://github.com/Timendus/chip8-test-suite)
