// 창 없이 ROM을 실행하고 화면을 텍스트로 출력합니다. 테스트 ROM 결과 확인용입니다.
//
// cargo run --example headless -- <ROM> [frames] [chip8|schip] [0x1FF 값]
//
// Timendus chip8-test-suite는 0x1FF에 값을 써 두면 메뉴 선택을 건너뜁니다.
// (예: 5-quirks.ch8은 1 = CHIP-8, 2 = SUPER-CHIP modern)

use libemulators::chip8::{HEIGHT, Interpreter, Quirks, WIDTH};

const INSTRUCTIONS_PER_FRAME: usize = 11;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: headless <ROM> [frames] [chip8|schip] [0x1FF value]");
        std::process::exit(2);
    };
    let frames: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(600);
    let quirks = match args.get(3).map(String::as_str) {
        Some("schip") => Quirks::schip(),
        _ => Quirks::chip8(),
    };

    let rom = std::fs::read(path).expect("cannot read ROM");
    let mut interpreter = Interpreter::with_quirks(rom, quirks).expect("invalid ROM");
    if let Some(value) = args.get(4).and_then(|s| s.parse().ok()) {
        interpreter.cpu.mmu.write_byte(0x1FF, value);
    }

    for _ in 0..frames {
        for _ in 0..INSTRUCTIONS_PER_FRAME {
            if let Err(e) = interpreter.step() {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        interpreter.update_timers();
    }

    let vram = interpreter.get_vram();
    for y in 0..HEIGHT {
        let line: String = (0..WIDTH)
            .map(|x| if vram[x + y * WIDTH] == 1 { '#' } else { '.' })
            .collect();
        println!("{line}");
    }
}
