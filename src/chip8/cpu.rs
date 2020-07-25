const N_VREGISTERS: usize = 16;
const STACK_SIZE: usize = 16;

pub struct CPU {
    stack: [u16; STACK_SIZE],
    v_registers: [u8; N_VREGISTERS],
    i_register: u16,
    vf_register: u16,
    delay_timer: u8,
    sound_timer: u8,
    program_counter: u16,
    stack_pointer: u8,
}

impl CPU {
    /// Create a new CPU instance.
    /// Every component is started at `0`.
    pub fn new() -> Self {
        CPU {
            stack: [0; 16],
            v_registers: [0; 16],
            i_register: 0,
            vf_register: 0,
            delay_timer: 0,
            sound_timer: 0,
            program_counter: 0,
            stack_pointer: 0,
        }
    }

    /// Return from a subroutine.
    ///
    /// Wiites the address on top of the stack to the program counter and then
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

    /// Skip the next instruction if the value in the register `r_idx` is equal to `value`.
    ///
    /// If the values are equal, the program counter is incremented by 2.
    fn skip_eq_value(&mut self, r_idx: u8, value: u8) {
        if self.v_registers[r_idx as usize] == value {
            self.program_counter += 2;
        }
    }

    /// Skip the next instruction if the value in the register `r_idx` is not equal to `value`.
    ///
    /// If the values are not equal, the program counter is incremented by 2.
    fn skip_neq_value(&mut self, r_idx: u8, value: u8) {
        if self.v_registers[r_idx as usize] != value {
            self.program_counter += 2;
        }
    }

    /// Skip the next instruction if the value in the register `x_idx` is equal to the value in the register `y_idx`.
    ///
    /// If the values are equal, the program counter is incremented by 2.
    fn skip_eq_regs(&mut self, x_idx: u8, y_idx: u8) {
        if self.v_registers[x_idx as usize] != self.v_registers[y_idx as usize] {
            self.program_counter += 2;
        }
    }

    /// Set the register `r_idx` to `value`.
    fn set_register(&mut self, r_idx: u8, value: u8) {
        self.v_registers[r_idx as usize] = value;
    }

    /// Add `value` to the current value of the register `r_idx`.
    fn add_register(&mut self, r_idx: u8, value: u8) {
        self.v_registers[r_idx as usize] += value;
    }

    /// Store the value of the register `y_idx` in the register `x_idx`.
    fn store_reg(&mut self, x_idx: u8, y_idx: u8) {
        self.v_registers[x_idx as usize] = self.v_registers[y_idx as usize];
    }

    /// Perform a bitwise **or** between the values of the registers `x_idx` and `y_idx`,
    /// then store the result in the register `x_idx`.
    fn or_reg(&mut self, x_idx: u8, y_idx: u8) {
        self.v_registers[x_idx as usize] |= self.v_registers[y_idx as usize];
    }

    /// Perform a bitwise **and** between the values of the registers `x_idx` and `y_idx`,
    /// then store the result in the register `x_idx`.
    fn and_reg(&mut self, x_idx: u8, y_idx: u8) {
        self.v_registers[x_idx as usize] &= self.v_registers[y_idx as usize];
    }

    /// Perform a bitwise **xor** between the values of the registers `x_idx` and `y_idx`,
    /// then store the result in the register `x_idx`.
    fn xor_reg(&mut self, x_idx: u8, y_idx: u8) {
        self.v_registers[x_idx as usize] ^= self.v_registers[y_idx as usize];
    }

    fn add_reg(&mut self, x_idx: u8, y_idx: u8) {
        let v: u16 = self.v_registers[x_idx as usize] as u16 + self.v_registers[y_idx as usize] as u16;
        if v >> 8 != 0 {
            self.vf_register = 1;
        } else {
            self.vf_register = 0;
        }
        self.v_registers[x_idx as usize] = (v & 0x00FF) as u8;
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
            (0x3, r_idx, _, _) => self.skip_eq_value(r_idx as u8, (op_3 | op_4) as u8),
            (0x4, r_idx, _, _) => self.skip_neq_value(r_idx as u8, (op_3 | op_4) as u8),
            (0x5, x_idx, y_idx, 0x0) => self.skip_eq_regs(x_idx as u8, y_idx as u8),
            (0x6, r_idx, _, _) => self.set_register(r_idx as u8, (op_3 | op_4) as u8),
            (0x7, r_idx, _, _) => self.add_register(r_idx as u8, (op_3 | op_4) as u8),
            (0x8, x_idx, y_idx, 0x0) => self.store_reg(x_idx as u8, y_idx as u8),
            (0x8, x_idx, y_idx, 0x1) => self.or_reg(x_idx as u8, y_idx as u8),
            (0x8, x_idx, y_idx, 0x2) => self.and_reg(x_idx as u8, y_idx as u8),
            (0x8, x_idx, y_idx, 0x3) => self.xor_reg(x_idx as u8, y_idx as u8),
            (0x8, x_idx, y_idx, 0x4) => self.add_reg(x_idx as u8, y_idx as u8),
            (_, _, _, _) => {}
        }
    }
}
