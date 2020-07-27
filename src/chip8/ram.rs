pub struct RAM {
    memory: [u8; 4096],
}

impl RAM {
    /// Create a new RAM instance.
    pub fn new() -> Self {
        RAM { memory: [0; 4096] }
    }

    /// Read a single byte from memory.
    pub fn read(&self, addr: usize) -> u8 {
        return self.memory[addr];
    }

    /// Write a single byte to memory.
    /// Panics if the `addr` value is less than `0x200`.
    pub fn write(&mut self, addr: usize, value: u8) {
        if addr <= 0x1FF {
            panic!("invalid write at {}", addr);
        }
        self.memory[addr] = value;
    }
}
