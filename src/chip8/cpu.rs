use crate::chip8::ram;
use rand::prelude::*;

const N_VREGISTERS: usize = 16;
const STACK_SIZE: usize = 16;
const VF: usize = 0xF;
const V0: usize = 0x0;

pub struct CPU {
    stack: [u16; STACK_SIZE],
    v_reg: [u8; N_VREGISTERS],
    i_reg: u16,
    delay_timer: u8,
    sound_timer: u8,
    program_counter: u16,
    stack_pointer: u8,
    rng: ThreadRng,
    ram: ram::RAM,
}

impl CPU {
    /// Create a new CPU instance.
    /// Every component is started at `0`.
    pub fn new() -> Self {
        CPU {
            stack: [0; STACK_SIZE],
            v_reg: [0; N_VREGISTERS],
            i_reg: 0,
            delay_timer: 0,
            sound_timer: 0,
            program_counter: 0,
            stack_pointer: 0,
            rng: rand::thread_rng(),
            ram: ram::RAM::new(),
        }
    }

    /// Return from a subroutine.
    ///
    /// Writes the address on top of the stack to the program counter and then
    /// it subtracts `1` from the stack pointer.
    fn ret_subroutine(&mut self) {
        self.program_counter = self.stack[self.stack_pointer as usize];
        self.stack_pointer -= 1;
    }

    /// Jump to a memory address.
    ///
    /// Writes `addr` to the program counter.
    fn jmp_addr(&mut self, addr: u16) {
        self.program_counter = addr;
    }

    /// Call subroutine.
    ///
    /// Increments the stack pointer,
    /// then puts the current program counter on top of the stack, finally,
    /// the program counter is set to `addr`.
    fn call_subroutine(&mut self, addr: u16) {
        self.stack_pointer += 1;
        self.stack[self.stack_pointer as usize] = self.program_counter;
        self.program_counter = addr;
    }

    /// Skip the next instruction if the value in the register `x_idx` is equal to `value`.
    ///
    /// If the values are equal, the program counter is incremented by 2.
    fn skip_eq_value(&mut self, x_idx: u8, value: u8) {
        if self.v_reg[x_idx as usize] == value {
            self.program_counter += 2;
        }
    }

    /// Skip the next instruction if the value in the register `x_idx` is not equal to `value`.
    ///
    /// If the values are not equal, the program counter is incremented by 2.
    fn skip_neq_value(&mut self, x_idx: u8, value: u8) {
        if self.v_reg[x_idx as usize] != value {
            self.program_counter += 2;
        }
    }

    /// Skip the next instruction if the value in the register `x_idx` is equal to the value in the register `y_idx`.
    ///
    /// If the values are equal, the program counter is incremented by 2.
    fn skip_eq_xy(&mut self, x_idx: u8, y_idx: u8) {
        if self.v_reg[x_idx as usize] == self.v_reg[y_idx as usize] {
            self.program_counter += 2;
        }
    }

    /// Set the register `x_idx` to `value`.
    fn set_x_value(&mut self, x_idx: u8, value: u8) {
        self.v_reg[x_idx as usize] = value;
    }

    /// Add `value` to the current value of the register `x_idx`.
    fn add_x_value(&mut self, x_idx: u8, value: u8) {
        self.v_reg[x_idx as usize] += value;
    }

    /// Store the value of the register `y_idx` in the register `x_idx`.
    fn store_xy(&mut self, x_idx: u8, y_idx: u8) {
        self.v_reg[x_idx as usize] = self.v_reg[y_idx as usize];
    }

    /// Perform a bitwise *OR* between the values of the registers `x_idx` and `y_idx`,
    /// then store the result in the register `x_idx`.
    fn or_xy(&mut self, x_idx: u8, y_idx: u8) {
        self.v_reg[x_idx as usize] |= self.v_reg[y_idx as usize];
    }

    /// Perform a bitwise *AND* between the values of the registers `x_idx` and `y_idx`,
    /// then store the result in the register `x_idx`.
    fn and_xy(&mut self, x_idx: u8, y_idx: u8) {
        self.v_reg[x_idx as usize] &= self.v_reg[y_idx as usize];
    }

    /// Perform a bitwise *XOR* between the values of the registers `x_idx` and `y_idx`,
    /// then store the result in the register `x_idx`.
    fn xor_xy(&mut self, x_idx: u8, y_idx: u8) {
        self.v_reg[x_idx as usize] ^= self.v_reg[y_idx as usize];
    }

    /// Add the values in registers `x_idx` and `y_idx`, storing the result in `x_idx`.
    ///
    /// If the result is greater than `255` then the `VF` register is set to `1`,
    /// otherwise, it is set to `0`.
    /// The lower 8 bits of the result are kept and stored in the register `x_idx`.
    fn add_xy(&mut self, x_idx: u8, y_idx: u8) {
        let v: u16 = self.v_reg[x_idx as usize] as u16 + self.v_reg[y_idx as usize] as u16;
        if v >> 8 != 0 {
            self.v_reg[VF] = 1;
        } else {
            self.v_reg[VF] = 0;
        }
        self.v_reg[x_idx as usize] = (v & 0x00FF) as u8;
    }

    /// Subtract the values in registers `x_idx` and `y_idx`, storing the result in `x_idx`.
    ///
    /// If the value of the register `x_idx` is greater than `y_idx`,
    /// then the `VF` register is set to `1`, otherwise it is set to `0`.
    fn sub_xy(&mut self, x_idx: u8, y_idx: u8) {
        if self.v_reg[x_idx as usize] > self.v_reg[y_idx as usize] {
            self.v_reg[VF] = 1;
        } else {
            self.v_reg[VF] = 0;
        }
        self.v_reg[x_idx as usize] -= self.v_reg[y_idx as usize];
    }

    /// Perform a bitwise-shift *right* on the value of the register `x_idx`.
    ///
    /// If the least-significant bit of the register `x_idx` is `1` then VF is set to `1`,
    /// otherwise it is set to `0`.
    fn shr_x(&mut self, x_idx: u8) {
        self.v_reg[VF] = self.v_reg[x_idx as usize] & 0x1;
        self.v_reg[x_idx as usize] >>= 1;
    }

    /// Subtracts the values in registers `y_idx` and `x_idx`, storing the result in `x_idx`.
    ///
    /// If the value of the register `y_idx` is greater than `x_idx`,
    /// then the `VF` register is set to `1`, otherwise it is set to `0`.
    fn subn_xy(&mut self, x_idx: u8, y_idx: u8) {
        if self.v_reg[y_idx as usize] > self.v_reg[x_idx as usize] {
            self.v_reg[VF] = 1;
        } else {
            self.v_reg[VF] = 0;
        }
        self.v_reg[x_idx as usize] = self.v_reg[y_idx as usize] - self.v_reg[x_idx as usize];
    }

    /// Perform a bitwise-shift *left* on the value of the register `x_idx`.
    ///
    /// If the most-significant bit of the register `x_idx` is `1` then VF is set to `1`,
    /// otherwise it is set to `0`.
    fn shl_x(&mut self, x_idx: u8) {
        self.v_reg[VF] = self.v_reg[x_idx as usize] >> 7;
        self.v_reg[x_idx as usize] <<= 1;
    }

    /// Skip the next instruction if the values in registers `x_idx` and `y_idx` are not equal.
    ///
    /// If the values are not equal, the program counter is incremented by `2`.
    fn skip_neq_xy(&mut self, x_idx: u8, y_idx: u8) {
        if self.v_reg[x_idx as usize] != self.v_reg[y_idx as usize] {
            self.program_counter += 2;
        }
    }

    /// Set the value of the `I` register to `addr`.
    fn set_i(&mut self, addr: u16) {
        self.i_reg = addr;
    }

    /// Jump to the location `addr + V0`.
    ///
    /// The program counter is set to the resulting sum of `addr` and `V0`.
    fn jmp_addr_offset(&mut self, addr: u16) {
        self.program_counter = addr + self.v_reg[V0] as u16;
    }

    /// Generate a random value between `0` and `255`,
    /// and perform a bitwise *AND* between the generated number and `value`.
    /// The operation result is stored in the register `x_idx`.
    fn rnd_and(&mut self, x_idx: u8, value: u8) {
        let r_num: u8 = self.rng.gen();
        self.v_reg[x_idx as usize] = r_num & value;
    }

    /// TODO
    fn draw(&mut self, x_idx: u8, y_idx: u8, n: u8) {
        let start = self.i_reg as usize;
        let end = start + (n as usize);
        for addr in start..end {
            // self.ram.read(addr)
        }
    }

    /// TODO
    fn skip_key_pressed(&mut self, x_idx: u8) {
        unimplemented!()
    }

    /// TODO
    fn skip_key_not_pressed(&mut self, x_idx: u8) {
        unimplemented!()
    }

    /// Read the value from the delay timer into the register `x_idx`.
    fn read_delay_timer(&mut self, x_idx: u8) {
        self.v_reg[x_idx as usize] = self.delay_timer;
    }

    /// TODO
    fn wait_keypress(&mut self, x_idx: u8) {
        unimplemented!()
    }

    /// Write the value of the register `x_idx` into the delay timer.
    fn set_delay_timer(&mut self, x_idx: u8) {
        self.delay_timer = self.v_reg[x_idx as usize];
    }

    /// Write the value of the register `x_idx` into the sound timer.
    fn set_sound_timer(&mut self, x_idx: u8) {
        self.sound_timer = self.v_reg[x_idx as usize];
    }

    /// Increment the value of the `I` register by the value in the `x_idx` register.
    fn add_i(&mut self, x_idx: u8) {
        self.i_reg += self.v_reg[x_idx as usize] as u16;
    }

    /// TODO
    fn set_i_digit(&mut self, x_idx: u8) {
        unimplemented!()
    }

    /// TODO
    fn store_bcd(&mut self, x_idx: u8) {
        unimplemented!()
    }

    /// Write registers from `0` to `x_idx` (inclusive), to memory.
    ///
    /// Writing starts at the address in `I` and progresses in increments (`I`, `I+1`, `I+2`, `...`).
    fn store_registers(&mut self, x_idx: u8) {
        for idx in 0..=(x_idx as usize) {
            self.ram.write(self.i_reg as usize + idx, self.v_reg[idx]);
        }
    }

    /// Read from memory to registers `0` to `x_idx` (inclusive).
    ///
    /// Reading starts at the address in `I` and progresses in increments (`I`, `I+1`, `I+2`, `...`).
    fn read_registers(&mut self, x_idx: u8) {
        for idx in 0..=(x_idx as usize) {
            self.v_reg[idx] = self.ram.read(self.i_reg as usize + idx);
        }
    }

    fn cycle(&mut self, opcode: u16) {
        let op_1 = (opcode & 0xF000) >> 12;
        let op_2 = (opcode & 0x0F00) >> 8;
        let op_3 = (opcode & 0x00F0) >> 4;
        let op_4 = opcode & 0x000F;

        match (op_1, op_2, op_3, op_4) {
            (0x0, 0x0, 0xE, 0x0) => {
                // clear the display
            }
            (0x0, 0x0, 0xE, 0xE) => self.ret_subroutine(),
            (0x1, _, _, _) => self.jmp_addr(op_2 | op_3 | op_4),
            (0x2, _, _, _) => self.call_subroutine(op_2 | op_3 | op_4),
            (0x3, x_idx, _, _) => self.skip_eq_value(x_idx as u8, (op_3 | op_4) as u8),
            (0x4, x_idx, _, _) => self.skip_neq_value(x_idx as u8, (op_3 | op_4) as u8),
            (0x5, x_idx, y_idx, 0x0) => self.skip_eq_xy(x_idx as u8, y_idx as u8),
            (0x6, x_idx, _, _) => self.set_x_value(x_idx as u8, (op_3 | op_4) as u8),
            (0x7, x_idx, _, _) => self.add_x_value(x_idx as u8, (op_3 | op_4) as u8),
            (0x8, x_idx, y_idx, 0x0) => self.store_xy(x_idx as u8, y_idx as u8),
            (0x8, x_idx, y_idx, 0x1) => self.or_xy(x_idx as u8, y_idx as u8),
            (0x8, x_idx, y_idx, 0x2) => self.and_xy(x_idx as u8, y_idx as u8),
            (0x8, x_idx, y_idx, 0x3) => self.xor_xy(x_idx as u8, y_idx as u8),
            (0x8, x_idx, y_idx, 0x4) => self.add_xy(x_idx as u8, y_idx as u8),
            (0x8, x_idx, y_idx, 0x5) => self.sub_xy(x_idx as u8, y_idx as u8),
            (0x8, x_idx, _, 0x6) => self.shr_x(x_idx as u8),
            (0x8, x_idx, y_idx, 0x7) => self.subn_xy(x_idx as u8, y_idx as u8),
            (0x8, x_idx, _, 0xE) => self.shl_x(x_idx as u8),
            (0x9, x_idx, y_idx, 0x0) => self.skip_neq_xy(x_idx as u8, y_idx as u8),
            (0xA, _, _, _) => self.set_i(op_2 | op_3 | op_4),
            (0xB, _, _, _) => self.jmp_addr_offset(op_2 | op_3 | op_4),
            (0xC, x_idx, _, _) => self.rnd_and(x_idx as u8, (op_3 | op_4) as u8),
            (0xD, x_idx, y_idx, n) => self.draw(x_idx as u8, y_idx as u8, n as u8),
            (0xE, x_idx, 0x9, 0xE) => self.skip_key_pressed(x_idx as u8),
            (0xE, x_idx, 0xA, 0x1) => self.skip_key_not_pressed(x_idx as u8),
            (0xF, x_idx, 0x0, 0x7) => self.read_delay_timer(x_idx as u8),
            (0xF, x_idx, 0x0, 0xA) => self.wait_keypress(x_idx as u8),
            (0xF, x_idx, 0x1, 0x5) => self.set_delay_timer(x_idx as u8),
            (0xF, x_idx, 0x1, 0x8) => self.set_sound_timer(x_idx as u8),
            (0xF, x_idx, 0x1, 0xE) => self.add_i(x_idx as u8),
            (0xF, x_idx, 0x2, 0x9) => self.set_i_digit(x_idx as u8),
            (0xF, x_idx, 0x3, 0x3) => self.store_bcd(x_idx as u8),
            (0xF, x_idx, 0x5, 0x5) => self.store_registers(x_idx as u8),
            (0xF, x_idx, 0x6, 0x5) => self.read_registers(x_idx as u8),
            (_, _, _, _) => panic!("unknown instruction"),
        }
    }
}

#[cfg(test)]
mod cpu_tests {
    use super::*;

    #[test]
    fn call_subroutine() {
        let mut cpu = CPU::new();
        let addr = 0x200;
        cpu.call_subroutine(addr);
        assert_eq!(cpu.stack_pointer, 1);
        assert_eq!(cpu.stack[cpu.stack_pointer as usize], 0);
        assert_eq!(cpu.program_counter, addr);
    }

    #[test]
    fn return_subroutine() {
        let mut cpu = CPU::new();
        cpu.call_subroutine(0x200);
        cpu.ret_subroutine();
        assert_eq!(cpu.program_counter, 0);
        assert_eq!(cpu.stack_pointer, 0);
    }

    #[test]
    fn jump_to_address() {
        let mut cpu = CPU::new();
        let addr = 0x200;
        cpu.jmp_addr(addr);
        assert_eq!(cpu.program_counter, addr);
    }

    #[test]
    fn skip_if_register_eq_value() {
        let mut cpu = CPU::new();
        let old_pc = cpu.program_counter;
        cpu.v_reg[0] = 128;
        cpu.skip_eq_value(0, 127);
        assert_eq!(cpu.program_counter, old_pc);
        cpu.skip_eq_value(0, 128);
        assert_eq!(cpu.program_counter, old_pc + 2);
    }

    #[test]
    fn skip_if_register_neq_value() {
        let mut cpu = CPU::new();
        let old_pc = cpu.program_counter;
        cpu.v_reg[0] = 128;
        cpu.skip_neq_value(0, 128);
        assert_eq!(cpu.program_counter, old_pc);
        cpu.skip_neq_value(0, 127);
        assert_eq!(cpu.program_counter, old_pc + 2);
    }

    #[test]
    fn skip_if_register_eq_xy() {
        let mut cpu = CPU::new();
        let old_pc = cpu.program_counter;
        cpu.v_reg[0] = 128;
        cpu.v_reg[7] = 128;
        cpu.v_reg[15] = 127;
        cpu.skip_eq_xy(0, 15);
        assert_eq!(cpu.program_counter, old_pc);
        cpu.skip_eq_xy(0, 7);
        assert_eq!(cpu.program_counter, old_pc + 2);
    }

    #[test]
    fn skip_if_register_neq_xy() {
        let mut cpu = CPU::new();
        let old_pc = cpu.program_counter;
        cpu.v_reg[0] = 128;
        cpu.v_reg[7] = 128;
        cpu.v_reg[15] = 127;
        cpu.skip_neq_xy(0, 7);
        assert_eq!(cpu.program_counter, old_pc);
        cpu.skip_neq_xy(0, 15);
        assert_eq!(cpu.program_counter, old_pc + 2);
    }

    #[test]
    fn set_store_register() {
        let mut cpu = CPU::new();
        cpu.set_x_value(0, 128);
        assert_eq!(cpu.v_reg[0], 128);
        cpu.store_xy(0, 1);
        assert_eq!(cpu.v_reg[0], 0);
    }

    #[test]
    fn add_value_to_register() {
        let mut cpu = CPU::new();
        cpu.set_x_value(0, 128);
        assert_eq!(cpu.v_reg[0], 128);
        cpu.add_x_value(0, 127);
        assert_eq!(cpu.v_reg[0], 255);
    }
}
