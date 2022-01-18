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
}

impl Mmu {
    /// Create a new `size` long memory space
    pub fn new(size: usize) -> Self {
        Self {
            memory:      vec![0; size],
            permissions: vec![0; size],
            alloc_base:  VAddr(0x0),
        }
    }

    /// Allocate a region in memory
    pub fn allocate(&mut self, size: usize) -> Option<VAddr> {
        // Padding to align the memory correctly. This helps cache friendliness
        let mem_align = 0xf;
        let align_pad = (size + mem_align) & !mem_align;

        // Update the allocation base
        let cur_base  = VAddr(self.alloc_base.0);
        let next_base = VAddr(cur_base.0.checked_add(align_pad)?);

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
