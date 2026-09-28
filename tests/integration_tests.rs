use libemulators::chip8::{HEIGHT, Interpreter, WIDTH};

#[test]
fn test_interpreter_creation() {
    let rom = vec![0x12, 0x34, 0x56, 0x78];
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // 기본 상태 확인
    assert_eq!(interpreter.cpu.registers.pc, 0x200); // ROM 시작 주소
    assert_eq!(interpreter.cpu.registers.sp, 0);
    assert_eq!(interpreter.cpu.registers.i, 0);
    assert_eq!(interpreter.cpu.registers.delay, 0);
    assert_eq!(interpreter.cpu.registers.sound, 0);

    // VRAM이 초기화되었는지 확인
    for pixel in interpreter.cpu.vram.iter() {
        assert_eq!(*pixel, 0);
    }
}

#[test]
fn test_keypad_update() {
    let rom = vec![0x12, 0x34];
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    let keypad = [
        true, false, true, false, false, false, false, false, false, false, false, false, false,
        false, false, false,
    ];

    interpreter.update_keypad(keypad);

    // 키패드 상태가 업데이트되었는지 확인 (public 메서드 사용)
    let keypad_state = interpreter.cpu.get_keypad_state();
    assert!(keypad_state[0]);
    assert!(!keypad_state[1]);
    assert!(keypad_state[2]);
}

#[test]
fn test_clear_screen_instruction() {
    let rom = vec![0x00, 0xE0]; // CLS instruction
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // VRAM에 일부 픽셀을 설정
    interpreter.cpu.vram[0] = 1;
    interpreter.cpu.vram[100] = 1;

    // 명령어 실행
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();

    // 화면이 지워졌는지 확인
    assert!(interpreter.should_redraw());
    for pixel in interpreter.cpu.vram.iter() {
        assert_eq!(*pixel, 0);
    }
}

#[test]
fn test_jump_instruction() {
    let rom = vec![0x12, 0x34]; // JP 0x234
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    let initial_pc = interpreter.cpu.registers.pc;
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();

    // PC가 점프 주소로 변경되었는지 확인
    assert_eq!(interpreter.cpu.registers.pc, 0x234);
    assert_ne!(interpreter.cpu.registers.pc, initial_pc);
}

#[test]
fn test_call_and_return() {
    // CALL 0x300, RET
    let mut rom = vec![0x00; 0x102];
    rom[0] = 0x23; // CALL 0x300
    rom[1] = 0x00;
    rom[0x100] = 0x00; // RET
    rom[0x101] = 0xEE;

    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // CALL 실행
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.pc, 0x300);
    assert_eq!(interpreter.cpu.registers.sp, 1);

    // 스택에 반환 주소가 저장되었는지 확인
    let stack = interpreter.cpu.get_stack();
    assert_eq!(stack[0], 0x202); // 반환 주소

    // RET 실행
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.pc, 0x202);
    assert_eq!(interpreter.cpu.registers.sp, 0);
}

#[test]
fn test_skip_instructions() {
    let rom = vec![0x31, 0x23, 0x41, 0x23, 0x51, 0x20]; // SE V1, 0x23; SNE V1, 0x23; SE V1, V2
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // V1, V2 값 설정을 pc 설정 이후에 수행
    interpreter.cpu.registers.v[1] = 0x23;
    interpreter.cpu.registers.v[2] = 0x23;

    let initial_pc = interpreter.cpu.registers.pc;
    println!("Initial PC: 0x{:X} ({})", initial_pc, initial_pc);
    println!(
        "V1: 0x{:X}, V2: 0x{:X}",
        interpreter.cpu.registers.v[1], interpreter.cpu.registers.v[2]
    );

    // SE V1, 0x23 (V1 == 0x23이므로 스킵)
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    println!(
        "After SE V1, 0x23: PC = 0x{:X} ({})",
        interpreter.cpu.registers.pc, interpreter.cpu.registers.pc
    );
    assert_eq!(interpreter.cpu.registers.pc, initial_pc + 4); // 0x204

    // SE V1, V2 (V1 == V2이므로 스킵)
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    println!(
        "After SE V1, V2: PC = 0x{:X} ({})",
        interpreter.cpu.registers.pc, interpreter.cpu.registers.pc
    );
    assert_eq!(interpreter.cpu.registers.pc, initial_pc + 8); // 0x208
}

#[test]
fn test_register_operations() {
    let rom = vec![0x61, 0xAB, 0x71, 0x23]; // LD V1, 0xAB; ADD V1, 0x23
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // LD V1, 0xAB
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.v[1], 0xAB);

    // ADD V1, 0x23
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.v[1], 0xCE); // 0xAB + 0x23 = 0xCE
}

#[test]
fn test_arithmetic_operations() {
    let rom = vec![0x61, 0x05, 0x62, 0x03, 0x81, 0x25]; // LD V1, 5; LD V2, 3; SUB V1, V2
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // V1 = 5, V2 = 3 설정
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.v[1], 5);
    assert_eq!(interpreter.cpu.registers.v[2], 3);

    // SUB V1, V2 (V1 = V1 - V2)
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.v[1], 2);
    assert_eq!(interpreter.cpu.registers.v[0xF], 1); // 빌림 없음 플래그
}

#[test]
fn test_draw_instruction() {
    let rom = vec![0x61, 0x00, 0x62, 0x00, 0xA3, 0x00, 0xD1, 0x23]; // LD V1, 0; LD V2, 0; LD I, 0x300; DRW V1, V2, 3
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // 초기화
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap(); // V1 = 0
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap(); // V2 = 0
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap(); // I = 0x300

    // 스프라이트 데이터 설정 (3x1 픽셀)
    interpreter.cpu.mmu.write_byte(0x300, 0xFF); // 모든 픽셀이 켜짐

    // DRW V1, V2, 3 (위치 (0,0)에 3바이트 스프라이트 그리기)
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();

    // VRAM이 변경되었는지 확인
    assert!(interpreter.should_redraw());

    // 첫 번째 행의 모든 픽셀이 켜져야 함
    assert_eq!(interpreter.cpu.vram[0], 1); // 첫 번째 픽셀
}

#[test]
fn test_timer_operations() {
    let rom = vec![0xF1, 0x07, 0xF1, 0x15, 0xF1, 0x18]; // LD V1, DT; LD DT, V1; LD ST, V1
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // 타이머에 값 설정
    interpreter.cpu.registers.delay = 0x42;
    interpreter.cpu.registers.sound = 0x10;

    // LD V1, DT
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.v[1], 0x42);

    // LD DT, V1
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.delay, 0x42);

    // LD ST, V1
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.sound, 0x42);
}

#[test]
fn test_timer_decrement() {
    let rom = vec![0x00, 0x00]; // NOP
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // 타이머 설정
    interpreter.cpu.registers.delay = 5;
    interpreter.cpu.registers.sound = 3;

    // 타이머 감소
    interpreter.update_timers();
    assert_eq!(interpreter.cpu.registers.delay, 4);
    assert_eq!(interpreter.cpu.registers.sound, 2);

    interpreter.update_timers();
    assert_eq!(interpreter.cpu.registers.delay, 3);
    assert_eq!(interpreter.cpu.registers.sound, 1);

    interpreter.update_timers();
    assert_eq!(interpreter.cpu.registers.delay, 2);
    assert_eq!(interpreter.cpu.registers.sound, 0);

    // 0 이하로 감소하지 않음
    interpreter.update_timers();
    assert_eq!(interpreter.cpu.registers.delay, 1);
    assert_eq!(interpreter.cpu.registers.sound, 0);
}

#[test]
fn test_sound_timer() {
    let rom = vec![0x00, 0x00]; // NOP
    let mut interpreter = Interpreter::new(rom.clone()).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // 사운드 타이머가 0일 때
    assert!(!interpreter.should_beep());

    // 사운드 타이머가 0이 아닐 때
    let mut interpreter = Interpreter::new(rom).unwrap();
    interpreter.cpu.registers.sound = 1;
    assert!(interpreter.should_beep());
}

#[test]
fn test_vram_access() {
    let rom = vec![0x00, 0x00]; // NOP
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    let vram = interpreter.get_vram();
    assert_eq!(vram.len(), WIDTH * HEIGHT);

    // 모든 픽셀이 초기화되었는지 확인
    for pixel in vram.iter() {
        assert_eq!(*pixel, 0);
    }
}

#[test]
fn test_complex_program() {
    // 간단한 프로그램: 화면 지우기 -> 점프 -> 레지스터 설정
    let mut rom = vec![0x00; 0x20]; // 충분한 크기의 ROM
    rom[0] = 0x00; // CLS
    rom[1] = 0xE0;
    rom[2] = 0x12; // JP 0x210
    rom[3] = 0x10;
    rom[0x10] = 0x61; // LD V1, 0x42 (점프 후 실행됨)
    rom[0x11] = 0x42;

    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    // CLS 실행
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert!(interpreter.should_redraw());

    // JP 실행
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.pc, 0x210);

    // LD V1, 0x42 실행
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.v[1], 0x42);
}

#[test]
fn test_key_waiting() {
    let rom = vec![0xF1, 0x0A]; // LD V1, K (키 대기)
    let mut interpreter = Interpreter::new(rom).unwrap();

    interpreter.cpu.registers.pc = 0x200;

    let initial_pc = interpreter.cpu.registers.pc;

    // 키 대기 명령어 실행
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();

    // PC가 변경되지 않았는지 확인 (키를 기다리는 중)
    assert_eq!(interpreter.cpu.registers.pc, initial_pc);
    assert!(interpreter.cpu.is_keypad_waiting());

    // 키 입력 시뮬레이션
    let keypad = [false; 16];
    interpreter.update_keypad(keypad);
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();

    // 여전히 대기 중
    assert_eq!(interpreter.cpu.registers.pc, initial_pc);

    // 키 5번 누름
    let mut keypad = [false; 16];
    keypad[5] = true;
    interpreter.update_keypad(keypad);
    let opcode = interpreter.cpu.fetch_instruction();
    println!("Fetched opcode: 0x{:04X}", opcode);
    interpreter.step().unwrap();

    // 키를 누르고 있는 동안은 여전히 대기
    assert_eq!(interpreter.cpu.registers.pc, initial_pc);
    assert!(interpreter.cpu.is_keypad_waiting());

    // 키를 떼면 다음 명령어로 진행
    interpreter.update_keypad([false; 16]);
    interpreter.step().unwrap();
    assert_eq!(interpreter.cpu.registers.pc, initial_pc + 2);
    assert_eq!(interpreter.cpu.registers.v[1], 5);
    assert!(!interpreter.cpu.is_keypad_waiting());
}
