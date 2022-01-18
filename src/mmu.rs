/// A guest Virtual Address
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VAddr(pub usize);

/// Memory space of an emulator
pub struct Mmu {
    /// Guest memory address space
    memory: Vec<u8>,
}

impl Mmu {
    /// Create a new `size` long memory space
    pub fn new(size: usize) -> Self {
        Self {
            memory:     vec![0; size],
        }
    }

    /// Write bytes from `buf` to memory at `addr`
    pub fn write(&mut self, addr: VAddr, buf: &[u8]) -> Option<()> {
        self.memory.get_mut(addr.0..addr.0.checked_add(buf.len())?)?
            .copy_from_slice(&buf);
        Some(())
    }

    /// Reads bytes from memory at `addr` to `buf`
    pub fn read(&self, addr: VAddr, buf: &mut [u8]) -> Option<()> {
        buf.copy_from_slice(
            &self.memory.get(addr.0..addr.0.checked_add(buf.len())?)?
        );
        Some(())
    }
}
