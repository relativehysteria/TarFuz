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
}

fn main() {
    let mut emulator = Emulator::new(1024);
    let mut buf      = [0; 64];
    emulator.memory.write(VAddr(0x0), b"This will get overwritten");
    emulator.memory.write(VAddr(0x10), b"This will get overwritten");
    emulator.memory.read(VAddr(0x0), &mut buf);

    buf.iter().for_each(|&c| if c != 0 { print!("{}", c as char) });
    println!();
}
