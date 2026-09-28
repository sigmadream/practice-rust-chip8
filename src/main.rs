use clap::{Parser, ValueEnum};
use minifb::{Key, Scale, Window, WindowOptions};
use std::{error::Error, path::PathBuf, process::ExitCode};

use libemulators::chip8::{self, Quirks};

// 타이머와 화면 갱신은 60Hz
const FRAME_RATE: usize = 60;

const COLOR_ON: u32 = 0xFFFFFF;
const COLOR_OFF: u32 = 0x000000;

#[derive(Clone, Copy, ValueEnum)]
enum Platform {
    // 원본 COSMAC VIP CHIP-8
    Chip8,
    // SUPER-CHIP 계열 동작
    Schip,
}

impl Platform {
    fn quirks(self) -> Quirks {
        match self {
            Platform::Chip8 => Quirks::chip8(),
            Platform::Schip => Quirks::schip(),
        }
    }
}

#[derive(Parser)]
#[command(name = "emulators", about = "A CHIP-8 interpreter", version)]
struct Cli {
    #[arg(value_name = "ROM")]
    rom_name: PathBuf,
    // 60Hz 한 프레임당 실행할 명령어 수 (기본 11 = 약 660 명령어/초)
    #[arg(
        long,
        default_value_t = 11,
        help = "Instructions per frame (60 frames/sec)"
    )]
    speed: u32,
    #[arg(long, value_enum, default_value_t = Platform::Chip8, help = "Quirks preset")]
    platform: Platform,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Cli) -> Result<(), Box<dyn Error>> {
    let rom = std::fs::read(&args.rom_name)
        .map_err(|e| format!("cannot read ROM '{}': {e}", args.rom_name.display()))?;

    let mut interpreter = chip8::Interpreter::with_quirks(rom, args.platform.quirks())?;

    // Graphics
    let mut window = Window::new(
        &format!("Chip8 Emulator - {} - ESC to exit", args.rom_name.display()),
        chip8::WIDTH,
        chip8::HEIGHT,
        WindowOptions {
            borderless: false,
            resize: false,
            scale: Scale::X8,
            title: true,
            ..WindowOptions::default()
        },
    )?;
    // minifb 기본값(4ms, 약 250fps) 대신 60fps로 제한하여 타이머를 60Hz로 맞춤
    window.set_target_fps(FRAME_RATE);
    let mut buffer: Vec<u32> = vec![COLOR_OFF; chip8::WIDTH * chip8::HEIGHT];

    // Audio: 오디오 장치가 없어도 소리 없이 실행
    let audio = Beeper::new();
    if audio.is_none() {
        eprintln!("warning: audio device is not available, running without sound");
    }

    while window.is_open() && !window.is_key_down(Key::Escape) {
        interpreter.update_keypad(read_keypad(&window));

        for _ in 0..args.speed {
            interpreter.step()?;
        }

        if interpreter.should_redraw() {
            for (pixel, &on) in buffer.iter_mut().zip(interpreter.get_vram()) {
                *pixel = if on == 1 { COLOR_ON } else { COLOR_OFF };
            }
            interpreter.clear_redraw();
        }

        if let Some(beeper) = &audio {
            beeper.set(interpreter.should_beep());
        }

        interpreter.update_timers();

        window.update_with_buffer(&buffer, chip8::WIDTH, chip8::HEIGHT)?;
    }

    Ok(())
}

struct Beeper {
    // stream은 sink가 살아있는 동안 유지되어야 함
    _stream: rodio::OutputStream,
    sink: rodio::Sink,
}

impl Beeper {
    fn new() -> Option<Self> {
        let (stream, handle) = rodio::OutputStream::try_default().ok()?;
        let sink = rodio::Sink::try_new(&handle).ok()?;
        sink.append(rodio::source::SineWave::new(400.0));
        sink.pause();
        Some(Beeper {
            _stream: stream,
            sink,
        })
    }

    fn set(&self, on: bool) {
        if on {
            self.sink.play();
        } else {
            self.sink.pause();
        }
    }
}

fn read_keypad(window: &Window) -> [bool; 16] {
    // 1 2 3 C -> 1 2 3 4
    // 4 5 6 D -> Q W E R
    // 7 8 9 E -> A S D F
    // A 0 B F -> Z X C V
    const KEYMAP: [Key; 16] = [
        Key::X,    // 0
        Key::Key1, // 1
        Key::Key2, // 2
        Key::Key3, // 3
        Key::Q,    // 4
        Key::W,    // 5
        Key::E,    // 6
        Key::A,    // 7
        Key::S,    // 8
        Key::D,    // 9
        Key::Z,    // A
        Key::C,    // B
        Key::Key4, // C
        Key::R,    // D
        Key::F,    // E
        Key::V,    // F
    ];

    KEYMAP.map(|key| window.is_key_down(key))
}
