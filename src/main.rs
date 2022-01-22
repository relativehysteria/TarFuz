pub mod mmu;

use mmu::{Mmu, VAddr};

/// State of the emulated system
struct Emulator {
    /// Memory for the emulator
    pub memory: Mmu,
}

impl Emulator {
    /// Create a new emulator with a `size` long memory space
    pub fn new(size: usize) -> Self {
        Self {
            memory: Mmu::new(size),
        }
    }

    /// Forks the emulator
    pub fn fork(&self) -> Self {
        Self {
            memory: self.memory.fork(),
        }
    }
}

fn main() {
    let alloc = 4096;
    let mut orig_emulator = Emulator::new(alloc);
    let base = orig_emulator.memory.allocate(alloc).unwrap();

    let mut emulator = orig_emulator.fork();
    for i in 0..10_000_000 {
        emulator.memory.write(base, b"asdf").unwrap();
        emulator.memory.reset(&orig_emulator.memory);
    }
}
