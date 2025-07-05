use minifb::{Key, Scale, Window, WindowOptions};
use clap::Parser;
use std::{
    fs::File,
    io::Read,
};

use libemulators::chip8;

#[derive(Parser)]
#[command(name = "emulators", about = "A CHIP-8 interpreter", version = "0.1.0")]
struct Cli {
    #[arg(value_name = "ROM")]
    rom_name: std::path::PathBuf,
    #[arg(long, default_value = "5", help = "Execution speed")]
    speed: u8,
}

fn main() {
    // cli
    let args = Cli::parse();
    let rom_name = args.rom_name;
    let mut file = File::open(&rom_name).unwrap();
    let mut rom = Vec::new();
    file.read_to_end(&mut rom).unwrap();
    let speed = args.speed;

    // chip 8 생성
    let mut interpreter = chip8::Interpreter::new(rom);

    // Graphics
    let mut window = Window::new(
        format!("Chip8 Emulator - {} - ESC to exit", rom_name.to_str().unwrap()).as_str(),
        chip8::WIDTH,
        chip8::HEIGHT,
        WindowOptions {
            borderless: false,
            resize: false,
            scale: Scale::X8,
            title: true,
            ..WindowOptions::default()
        },
    ).unwrap_or_else(|e| { panic!("{}", e); });
    let mut buffer: Vec<u32> = vec![0; chip8::WIDTH * chip8::HEIGHT];


    // Audio
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&stream_handle).unwrap();
    let source = rodio::source::SineWave::new(400.0);
    sink.append(source);
    sink.pause();

    while  window.is_open() && !window.is_key_down(Key::Escape) {
        let mut redraw = false;

        for _ in 0..speed {
            interpreter.update_keypad(read_keypad(&window));
            interpreter.step();

            if interpreter.should_redraw() {
                redraw = true;
            }
        }

        if redraw {
            for x in 0..chip8::WIDTH {
                for y in 0..chip8::HEIGHT {
                    let i = x + (y * chip8::WIDTH);
                    buffer[i] = if interpreter.get_vram()[i] == 1 {
                        0xFFFFFF
                    } else {
                        0x0
                    };
                }
            }
        }

        if interpreter.should_beep() {
            sink.play();
        } else {
            sink.pause();
        }

        interpreter.update_timers();

        window.update_with_buffer(&buffer, chip8::WIDTH, chip8::HEIGHT).unwrap();
    }
}

fn read_keypad(window: &Window) -> [bool; 16] {
    // 1 2 3 C -> 1 2 3 4
    // 4 5 6 D -> Q W E R
    // 7 8 9 E -> A S D F
    // A 0 B F -> Z X C V
    let keypad: [bool; 16] = [
        window.is_key_down(Key::X),    // 0
        window.is_key_down(Key::Key1), // 1
        window.is_key_down(Key::Key2), // 2
        window.is_key_down(Key::Key3), // 3
        window.is_key_down(Key::Q),    // 4
        window.is_key_down(Key::W),    // 5
        window.is_key_down(Key::E),    // 6
        window.is_key_down(Key::A),    // 7
        window.is_key_down(Key::S),    // 8
        window.is_key_down(Key::D),    // 9
        window.is_key_down(Key::Z),    // A
        window.is_key_down(Key::C),    // B
        window.is_key_down(Key::Key4), // C
        window.is_key_down(Key::R),    // D
        window.is_key_down(Key::F),    // E
        window.is_key_down(Key::V),    // F
    ];

    keypad
}
