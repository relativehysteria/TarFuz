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
    let mut emulator = Emulator::new(1024*1024);
    let mut buf      = [0; 0x2a];
    let message      = b"This will get overwritten.";

    // Allocate some memory
    emulator.memory.allocate(1024*512+message.len());

    // Overwrite example
    emulator.memory.write(VAddr(0x0), message).unwrap();
    emulator.memory.write(VAddr(0x10), message).unwrap();

    // Write some more so we can check dirty memory
    emulator.memory.write(VAddr(1024*128), message).unwrap();
    emulator.memory.write(VAddr(1024*256), message).unwrap();
    emulator.memory.write(VAddr(1024*350), message).unwrap();
    emulator.memory.write(VAddr(1024*370), message).unwrap();
    emulator.memory.write(VAddr(1024*512), message).unwrap();

    emulator.memory.read(VAddr(0x0), &mut buf[..message.len()+0x10]).unwrap();

    println!("{}", buf.into_iter().map(|c| c as char).collect::<String>());
    println!("Dirty indexes: {:?}", emulator.memory.dirty_indexes);
    println!("Dirty bitmap : {:?}", emulator.memory.dirty_bitmap);
    emulator.memory.dirty_bitmap.iter().for_each(|x| println!("{:0128b}", x));
}
