use std::default::Default;

// Chip-8 CPU 구조체 정의
pub struct CPU {
    // 4KB 메모리
    memory: [u8; 4096],
    // 16개의 8비트 레지스터(V0~VF)
    v: [u8; 16],
    // 주소 레지스터 I
    i: u16,
    // 프로그램 카운터
    pc: u16,
    // 스택 (16단계)
    stack: [u16; 16],
    // 스택 포인터
    sp: u8,
}

impl Default for CPU {
    fn default() -> Self {
        Self {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: 0x200, // 일반적으로 0x200에서 프로그램 시작
            stack: [0; 16],
            sp: 0,
        }
    }
}

impl CPU {
    pub fn new() -> Self {
        Self::default()
    }

    // 테스트를 위한 메모리 설정 메서드
    pub fn set_memory_at(&mut self, address: usize, value: u8) {
        if address < 4096 {
            self.memory[address] = value;
        }
    }

    // 테스트를 위한 메모리 읽기 메서드
    pub fn get_memory_at(&self, address: usize) -> u8 {
        if address < 4096 {
            self.memory[address]
        } else {
            0
        }
    }

    // 테스트를 위한 레지스터 값 확인 메서드
    pub fn get_register(&self, reg: usize) -> u8 {
        if reg < 16 {
            self.v[reg]
        } else {
            0
        }
    }

    // 테스트를 위한 I 레지스터 값 확인 메서드
    pub fn get_i_register(&self) -> u16 {
        self.i
    }

    // 테스트를 위한 PC 값 확인 메서드
    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    // 테스트를 위한 스택 포인터 값 확인 메서드
    pub fn get_sp(&self) -> u8 {
        self.sp
    }

    // 현재 명령어를 가져오기
    fn fetch(&self) -> u16 {
        let high_byte = self.memory[self.pc as usize] as u16;
        let low_byte = self.memory[(self.pc + 1) as usize] as u16;
        (high_byte << 8) | low_byte
    }

    // 명령어 디코딩 및 실행
    pub fn execute_instruction(&mut self) {
        let opcode = self.fetch();
        
        // 명령어 타입별로 분기
        match opcode & 0xF000 {
            0x0000 => self.execute_0xxx(opcode),
            0x1000 => self.execute_1xxx(opcode),
            0x2000 => self.execute_2xxx(opcode),
            0x3000 => self.execute_3xxx(opcode),
            0x4000 => self.execute_4xxx(opcode),
            0x5000 => self.execute_5xxx(opcode),
            0x6000 => self.execute_6xxx(opcode),
            0x7000 => self.execute_7xxx(opcode),
            0x8000 => self.execute_8xxx(opcode),
            0x9000 => self.execute_9xxx(opcode),
            0xA000 => self.execute_Axxx(opcode),
            0xB000 => self.execute_Bxxx(opcode),
            0xC000 => self.execute_Cxxx(opcode),
            0xD000 => self.execute_Dxxx(opcode),
            0xE000 => self.execute_Exxx(opcode),
            0xF000 => self.execute_Fxxx(opcode),
            _ => panic!("Unknown opcode: 0x{:04X}", opcode),
        }
        
        // 프로그램 카운터 증가 (점프 명령어는 개별적으로 처리)
        self.pc += 2;
    }

    // 0xxx 명령어들
    fn execute_0xxx(&mut self, opcode: u16) {
        match opcode {
            0x00E0 => {
                // CLS: 화면 지우기
                println!("CLS: 화면을 지웁니다");
            }
            0x00EE => {
                // RET: 서브루틴에서 복귀
                if self.sp > 0 {
                    self.sp -= 1;
                    self.pc = self.stack[self.sp as usize];
                }
            }
            _ => panic!("Unknown 0xxx opcode: 0x{:04X}", opcode),
        }
    }

    // 1xxx 명령어들
    fn execute_1xxx(&mut self, opcode: u16) {
        // JP addr: 점프
        let addr = opcode & 0x0FFF;
        self.pc = addr;
    }

    // 2xxx 명령어들
    fn execute_2xxx(&mut self, opcode: u16) {
        // CALL addr: 서브루틴 호출
        let addr = opcode & 0x0FFF;
        if self.sp < 16 {
            self.stack[self.sp as usize] = self.pc;
            self.sp += 1;
            self.pc = addr;
        }
    }

    // 3xxx 명령어들
    fn execute_3xxx(&mut self, opcode: u16) {
        // SE Vx, byte: Vx == byte이면 다음 명령어 건너뛰기
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let byte = (opcode & 0x00FF) as u8;
        if self.v[x] == byte {
            self.pc += 2;
        }
    }

    // 4xxx 명령어들
    fn execute_4xxx(&mut self, opcode: u16) {
        // SNE Vx, byte: Vx != byte이면 다음 명령어 건너뛰기
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let byte = (opcode & 0x00FF) as u8;
        if self.v[x] != byte {
            self.pc += 2;
        }
    }

    // 5xxx 명령어들
    fn execute_5xxx(&mut self, opcode: u16) {
        // SE Vx, Vy: Vx == Vy이면 다음 명령어 건너뛰기
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        if self.v[x] == self.v[y] {
            self.pc += 2;
        }
    }

    // 6xxx 명령어들
    fn execute_6xxx(&mut self, opcode: u16) {
        // LD Vx, byte: Vx = byte
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let byte = (opcode & 0x00FF) as u8;
        self.v[x] = byte;
    }

    // 7xxx 명령어들
    fn execute_7xxx(&mut self, opcode: u16) {
        // ADD Vx, byte: Vx += byte
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let byte = (opcode & 0x00FF) as u8;
        self.v[x] = self.v[x].wrapping_add(byte);
    }

    // 8xxx 명령어들
    fn execute_8xxx(&mut self, opcode: u16) {
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let n = opcode & 0x000F;

        match n {
            0x0 => self.v[x] = self.v[y],                    // LD Vx, Vy
            0x1 => self.v[x] |= self.v[y],                   // OR Vx, Vy
            0x2 => self.v[x] &= self.v[y],                   // AND Vx, Vy
            0x3 => self.v[x] ^= self.v[y],                   // XOR Vx, Vy
            0x4 => {                                         // ADD Vx, Vy
                let sum = self.v[x] as u16 + self.v[y] as u16;
                self.v[0xF] = if sum > 255 { 1 } else { 0 };
                self.v[x] = sum as u8;
            }
            0x5 => {                                         // SUB Vx, Vy
                self.v[0xF] = if self.v[x] > self.v[y] { 1 } else { 0 };
                self.v[x] = self.v[x].wrapping_sub(self.v[y]);
            }
            0x6 => {                                         // SHR Vx
                self.v[0xF] = self.v[x] & 1;
                self.v[x] >>= 1;
            }
            0x7 => {                                         // SUBN Vx, Vy
                self.v[0xF] = if self.v[y] > self.v[x] { 1 } else { 0 };
                self.v[x] = self.v[y].wrapping_sub(self.v[x]);
            }
            0xE => {                                         // SHL Vx
                self.v[0xF] = (self.v[x] & 0x80) >> 7;
                self.v[x] <<= 1;
            }
            _ => panic!("Unknown 8xxx opcode: 0x{:04X}", opcode),
        }
    }

    // 9xxx 명령어들
    fn execute_9xxx(&mut self, opcode: u16) {
        // SNE Vx, Vy: Vx != Vy이면 다음 명령어 건너뛰기
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        if self.v[x] != self.v[y] {
            self.pc += 2;
        }
    }

    // Axxx 명령어들
    fn execute_Axxx(&mut self, opcode: u16) {
        // LD I, addr: I = addr
        let addr = opcode & 0x0FFF;
        self.i = addr;
    }

    // Bxxx 명령어들
    fn execute_Bxxx(&mut self, opcode: u16) {
        // JP V0, addr: PC = V0 + addr
        let addr = opcode & 0x0FFF;
        self.pc = self.v[0] as u16 + addr;
    }

    // Cxxx 명령어들
    fn execute_Cxxx(&mut self, opcode: u16) {
        // RND Vx, byte: Vx = random & byte
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let byte = (opcode & 0x00FF) as u8;
        let random = rand::random::<u8>();
        self.v[x] = random & byte;
    }

    // Dxxx 명령어들
    fn execute_Dxxx(&mut self, opcode: u16) {
        // DRW Vx, Vy, nibble: 스프라이트 그리기
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let height = opcode & 0x000F;
        println!("DRW: 스프라이트 그리기 (x={}, y={}, height={})", self.v[x], self.v[y], height);
    }

    // Exxx 명령어들
    fn execute_Exxx(&mut self, opcode: u16) {
        let x = ((opcode & 0x0F00) >> 8) as usize;
        match opcode & 0x00FF {
            0x9E => {
                // SKP Vx: 키가 눌렸으면 다음 명령어 건너뛰기
                println!("SKP: 키 {} 확인", self.v[x]);
            }
            0xA1 => {
                // SKNP Vx: 키가 안 눌렸으면 다음 명령어 건너뛰기
                println!("SKNP: 키 {} 확인", self.v[x]);
            }
            _ => panic!("Unknown Exxx opcode: 0x{:04X}", opcode),
        }
    }

    // Fxxx 명령어들
    fn execute_Fxxx(&mut self, opcode: u16) {
        let x = ((opcode & 0x0F00) >> 8) as usize;
        match opcode & 0x00FF {
            0x07 => {
                // LD Vx, DT: Vx = 딜레이 타이머
                println!("LD Vx, DT: 딜레이 타이머 값을 V{}에 로드", x);
            }
            0x0A => {
                // LD Vx, K: 키 입력 대기
                println!("LD Vx, K: 키 입력 대기");
            }
            0x15 => {
                // LD DT, Vx: 딜레이 타이머 = Vx
                println!("LD DT, Vx: V{} 값을 딜레이 타이머에 설정", x);
            }
            0x18 => {
                // LD ST, Vx: 사운드 타이머 = Vx
                println!("LD ST, Vx: V{} 값을 사운드 타이머에 설정", x);
            }
            0x1E => {
                // ADD I, Vx: I += Vx
                self.i += self.v[x] as u16;
            }
            0x29 => {
                // LD F, Vx: I = 스프라이트 주소
                println!("LD F, Vx: 스프라이트 주소 설정");
            }
            0x33 => {
                // LD B, Vx: BCD 변환
                println!("LD B, Vx: BCD 변환");
            }
            0x55 => {
                // LD [I], Vx: 메모리에 레지스터들 저장
                for i in 0..=x {
                    self.memory[self.i as usize + i] = self.v[i];
                }
            }
            0x65 => {
                // LD Vx, [I]: 메모리에서 레지스터들 로드
                for i in 0..=x {
                    self.v[i] = self.memory[self.i as usize + i];
                }
            }
            _ => panic!("Unknown Fxxx opcode: 0x{:04X}", opcode),
        }
    }
}

// 테스트 함수들
fn test_register_operations() {
    println!("\n=== 레지스터 연산 테스트 ===");
    let mut cpu = CPU::new();
    
    // LD V1, 0x42 (6x1x): V1 = 0x42
    cpu.set_memory_at(0x200, 0x61);
    cpu.set_memory_at(0x201, 0x42);
    cpu.execute_instruction();
    assert_eq!(cpu.get_register(1), 0x42);
    println!("✓ LD V1, 0x42: V1 = 0x{:02X}", cpu.get_register(1));
    
    // ADD V1, 0x10 (7x1x): V1 += 0x10
    cpu.set_memory_at(0x202, 0x71);
    cpu.set_memory_at(0x203, 0x10);
    cpu.execute_instruction();
    assert_eq!(cpu.get_register(1), 0x52);
    println!("✓ ADD V1, 0x10: V1 = 0x{:02X}", cpu.get_register(1));
}

fn test_arithmetic_operations() {
    println!("\n=== 산술 연산 테스트 ===");
    let mut cpu = CPU::new();
    
    // LD V2, 0x05 (6x2x): V2 = 0x05
    cpu.set_memory_at(0x200, 0x62);
    cpu.set_memory_at(0x201, 0x05);
    cpu.execute_instruction();
    
    // LD V3, 0x03 (6x3x): V3 = 0x03
    cpu.set_memory_at(0x202, 0x63);
    cpu.set_memory_at(0x203, 0x03);
    cpu.execute_instruction();
    
    // ADD V2, V3 (8x2x3x4): V2 += V3, VF = carry
    cpu.set_memory_at(0x204, 0x82);
    cpu.set_memory_at(0x205, 0x34);
    cpu.execute_instruction();
    assert_eq!(cpu.get_register(2), 0x08);
    assert_eq!(cpu.get_register(0xF), 0x00); // carry 없음
    println!("✓ ADD V2, V3: V2 = 0x{:02X}, VF = 0x{:02X}", cpu.get_register(2), cpu.get_register(0xF));
    
    // SUB V2, V3 (8x2x3x5): V2 -= V3, VF = borrow
    cpu.set_memory_at(0x206, 0x82);
    cpu.set_memory_at(0x207, 0x35);
    cpu.execute_instruction();
    assert_eq!(cpu.get_register(2), 0x05);
    assert_eq!(cpu.get_register(0xF), 0x01); // borrow 없음
    println!("✓ SUB V2, V3: V2 = 0x{:02X}, VF = 0x{:02X}", cpu.get_register(2), cpu.get_register(0xF));
}

fn test_logical_operations() {
    println!("\n=== 논리 연산 테스트 ===");
    let mut cpu = CPU::new();
    
    // LD V4, 0x0F (6x4x): V4 = 0x0F
    cpu.set_memory_at(0x200, 0x64);
    cpu.set_memory_at(0x201, 0x0F);
    cpu.execute_instruction();
    
    // LD V5, 0xF0 (6x5x): V5 = 0xF0
    cpu.set_memory_at(0x202, 0x65);
    cpu.set_memory_at(0x203, 0xF0);
    cpu.execute_instruction();
    
    // OR V4, V5 (8x4x5x1): V4 |= V5
    cpu.set_memory_at(0x204, 0x84);
    cpu.set_memory_at(0x205, 0x51);
    cpu.execute_instruction();
    assert_eq!(cpu.get_register(4), 0xFF);
    println!("✓ OR V4, V5: V4 = 0x{:02X}", cpu.get_register(4));
    
    // AND V4, V5 (8x4x5x2): V4 &= V5
    cpu.set_memory_at(0x206, 0x84);
    cpu.set_memory_at(0x207, 0x52);
    cpu.execute_instruction();
    assert_eq!(cpu.get_register(4), 0xF0);
    println!("✓ AND V4, V5: V4 = 0x{:02X}", cpu.get_register(4));
    
    // XOR V4, V5 (8x4x5x3): V4 ^= V5
    cpu.set_memory_at(0x208, 0x84);
    cpu.set_memory_at(0x209, 0x53);
    cpu.execute_instruction();
    assert_eq!(cpu.get_register(4), 0x00);
    println!("✓ XOR V4, V5: V4 = 0x{:02X}", cpu.get_register(4));
}

fn test_flow_control() {
    println!("\n=== 플로우 제어 테스트 ===");
    let mut cpu = CPU::new();
    
    // LD V6, 0x42 (6x6x): V6 = 0x42
    cpu.set_memory_at(0x200, 0x66);
    cpu.set_memory_at(0x201, 0x42);
    cpu.execute_instruction();
    
    // SE V6, 0x42 (3x6x): V6 == 0x42이면 다음 명령어 건너뛰기
    cpu.set_memory_at(0x202, 0x36);
    cpu.set_memory_at(0x203, 0x42);
    let pc_before = cpu.get_pc();
    cpu.execute_instruction();
    let pc_after = cpu.get_pc();
    assert_eq!(pc_after, pc_before + 4); // 2바이트 건너뜀
    println!("✓ SE V6, 0x42: PC = 0x{:04X} (건너뜀)", pc_after);
    
    // SNE V6, 0x41 (4x6x): V6 != 0x41이면 다음 명령어 건너뛰기
    cpu.set_memory_at(0x206, 0x46);
    cpu.set_memory_at(0x207, 0x41);
    let pc_before = cpu.get_pc();
    cpu.execute_instruction();
    let pc_after = cpu.get_pc();
    assert_eq!(pc_after, pc_before + 4); // 2바이트 건너뜀
    println!("✓ SNE V6, 0x41: PC = 0x{:04X} (건너뜀)", pc_after);
}

fn test_memory_operations() {
    println!("\n=== 메모리 연산 테스트 ===");
    let mut cpu = CPU::new();
    
    // LD I, 0x300 (Ax3x): I = 0x300
    cpu.set_memory_at(0x200, 0xA3);
    cpu.set_memory_at(0x201, 0x00);
    cpu.execute_instruction();
    assert_eq!(cpu.get_i_register(), 0x300);
    println!("✓ LD I, 0x300: I = 0x{:04X}", cpu.get_i_register());
    
    // LD V0, 0xAA (6x0x): V0 = 0xAA
    cpu.set_memory_at(0x202, 0x60);
    cpu.set_memory_at(0x203, 0xAA);
    cpu.execute_instruction();
    
    // LD V1, 0xBB (6x1x): V1 = 0xBB
    cpu.set_memory_at(0x204, 0x61);
    cpu.set_memory_at(0x205, 0xBB);
    cpu.execute_instruction();
    
    // LD [I], V1 (Fx5x): 메모리에 V0~V1 저장
    cpu.set_memory_at(0x206, 0xF1);
    cpu.set_memory_at(0x207, 0x55);
    cpu.execute_instruction();
    assert_eq!(cpu.get_memory_at(0x300), 0xAA);
    assert_eq!(cpu.get_memory_at(0x301), 0xBB);
    println!("✓ LD [I], V1: 메모리[0x300] = 0x{:02X}, 메모리[0x301] = 0x{:02X}", 
             cpu.get_memory_at(0x300), cpu.get_memory_at(0x301));
    
    // 레지스터 초기화
    cpu.v[0] = 0;
    cpu.v[1] = 0;
    
    // LD V1, [I] (Fx6x): 메모리에서 V0~V1 로드
    cpu.set_memory_at(0x208, 0xF1);
    cpu.set_memory_at(0x209, 0x65);
    cpu.execute_instruction();
    assert_eq!(cpu.get_register(0), 0xAA);
    assert_eq!(cpu.get_register(1), 0xBB);
    println!("✓ LD V1, [I]: V0 = 0x{:02X}, V1 = 0x{:02X}", 
             cpu.get_register(0), cpu.get_register(1));
}

fn test_stack_operations() {
    println!("\n=== 스택 연산 테스트 ===");
    let mut cpu = CPU::new();
    
    // CALL 0x400 (2x4x): 서브루틴 호출
    cpu.set_memory_at(0x200, 0x24);
    cpu.set_memory_at(0x201, 0x00);
    let pc_before = cpu.get_pc();
    cpu.execute_instruction();
    assert_eq!(cpu.get_pc(), 0x400);
    assert_eq!(cpu.get_sp(), 1);
    println!("✓ CALL 0x400: PC = 0x{:04X}, SP = {}", cpu.get_pc(), cpu.get_sp());
    
    // RET (0x00EE): 서브루틴 복귀
    cpu.set_memory_at(0x400, 0x00);
    cpu.set_memory_at(0x401, 0xEE);
    cpu.execute_instruction();
    assert_eq!(cpu.get_pc(), pc_before + 2);
    assert_eq!(cpu.get_sp(), 0);
    println!("✓ RET: PC = 0x{:04X}, SP = {}", cpu.get_pc(), cpu.get_sp());
}

fn main() {
    println!("=== Chip-8 CPU 테스트 시작 ===");
    
    // 모든 테스트 실행
    test_register_operations();
    test_arithmetic_operations();
    test_logical_operations();
    test_flow_control();
    test_memory_operations();
    test_stack_operations();
    
    println!("\n=== 모든 테스트 완료! ===");
    println!("CPU, 메모리, 명령어 디코더가 정상적으로 작동합니다.");
}
