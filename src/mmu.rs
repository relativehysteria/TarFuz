#![allow(dead_code)]

// Permission bit field
/// Write permission
const PERM_WRITE: u8 = 1 << 0;
/// Read permission
const PERM_READ:  u8 = 1 << 1;
/// Exec permission
const PERM_EXEC:  u8 = 1 << 2;

/// A guest Virtual Address
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VAddr(pub usize);

/// Memory space of an emulator
pub struct Mmu {
    /// Guest memory address space
    memory: Vec<u8>,

    /// Permissions of the corresponding memory.
    /// This doubles the memory footprint, I am aware
    permissions: Vec<u8>,

    /// Base `VAddr` of the next allocation
    alloc_base: VAddr,

    /// Memory is aligned to this base
    alignment: usize,
}

impl Mmu {
    /// Create a new `size` long memory space.
    ///
    /// Allocation base of the virtual memory is set to `0x0`.
    /// All memory is aligned to `0xf`.
    pub fn new(size: usize) -> Self {
        let alignment  = 0xf;
        let alloc_base = VAddr(0x0);

        let aligned_size = (size + alignment) & !alignment;
        Self {
            memory:      vec![0; aligned_size],
            permissions: vec![0; aligned_size],
            alloc_base,
            alignment,
        }
    }

    /// Returns the number `num` aligned to `self.alignment`
    pub fn align(&self, num: usize) -> usize {
        (num + self.alignment) & !self.alignment
    }


    /// Allocate a region in memory
    pub fn allocate(&mut self, size: usize) -> Option<VAddr> {
        // Update the allocation base
        let cur_base  = VAddr(self.alloc_base.0);
        let next_base = VAddr(cur_base.0.checked_add(self.align(size))?);

        // Don't allocate OOM
        if next_base.0 > self.memory.len() {
            return None;
        }

        // Mark the memory as writable
        self.set_permissions(cur_base, size, PERM_WRITE)?;

        self.alloc_base = next_base;
        Some(cur_base)
    }

    /// Set the permissions of a `size` long memory block starting from `addr`
    /// to `perm`
    pub fn set_permissions(&mut self, addr: VAddr,
                           size: usize, perm: u8) -> Option<()> {
        self.permissions.get_mut(addr.0..addr.0.checked_add(size)?)?
            .iter_mut().for_each(|x| *x = perm);
        Some(())
    }

    /// Write bytes from `buf` to memory at `addr`.
    /// The resulting bytes are set to be readable (`PERM_READ`)
    pub fn write(&mut self, addr: VAddr, buf: &[u8]) -> Option<()> {
        let perms =
            self.permissions.get_mut(addr.0..addr.0.checked_add(buf.len())?)?;

        // Check that we can write to memory
        if perms.iter().any(|x| (x & PERM_WRITE) == 0) {
            return None;
        }

        // Write the buffer to memory
        self.memory.get_mut(addr.0..addr.0.checked_add(buf.len())?)?
            .copy_from_slice(&buf);

        // Set it all to be readable
        perms.iter_mut().for_each(|x| *x |= PERM_READ);
        Some(())
    }

    /// Reads bytes from memory at `addr` to `buf`
    pub fn read(&self, addr: VAddr, buf: &mut [u8]) -> Option<()> {
        let perms =
            self.permissions.get(addr.0..addr.0.checked_add(buf.len())?)?;

        // Check that we can read from memory
        if perms.iter().any(|x| (x & PERM_READ) == 0) {
            return None;
        }

        // Read the memory
        buf.copy_from_slice(
            &self.memory.get(addr.0..addr.0.checked_add(buf.len())?)?
        );
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MSG: &[u8] = b"This is some text that is written into the memory.";

    #[test]
    fn byte_perfect_allocation() {
        let mut mem = Mmu::new(MSG.len());
        mem.allocate(MSG.len()).unwrap();
    }

    #[test]
    #[should_panic]
    fn out_of_memory() {
        let mut mem = Mmu::new(0x400);
        mem.allocate(0x100).unwrap(); // OK
        mem.allocate(0x100).unwrap(); // OK
        mem.allocate(0x100).unwrap(); // OK
        mem.allocate(0x100).unwrap(); // OK
        mem.allocate(0x1).unwrap();   // Panic
    }

    #[test]
    fn read_write() {
        let mut mem = Mmu::new(MSG.len());
        let mut buf = [0; MSG.len()];
        mem.allocate(MSG.len()).unwrap();
        mem.write(VAddr(0x0), MSG).unwrap();
        mem.read(VAddr(0x0), &mut buf).unwrap();
        assert!(buf[0..MSG.len()] == *MSG);
    }

    #[test]
    #[should_panic]
    fn read_uninitialized_memory() {
        let mem = Mmu::new(MSG.len());
        let mut buf = [0; MSG.len()];
        mem.read(VAddr(0x0), &mut buf).unwrap();
    }

    #[test]
    #[should_panic]
    fn write_unallocated_memory() {
        let mut mem = Mmu::new(MSG.len());
        mem.write(VAddr(0x0), MSG).unwrap();
    }
}
