#![allow(dead_code)]

/// Memory is aligned to this base.
const ALIGNMENT: usize = 0xf;

/// Size of a dirty block. Used for tracking memory which has been modified
/// since the emulator started running (either through initialization or through
/// `fork()`).
const DIRTY_BLOCK_SIZE: usize = 4096;

/// Dirty-Bitmap-Element BITS.
/// Number of bits in a single `dirty_bitmap` element
const DBE_BITS: usize = u128::BITS as usize;

// Permission bit field
/// Write permission
const PERM_WRITE: u8 = 1 << 0;
/// Read permission
const PERM_READ:  u8 = 1 << 1;
/// Exec permission
const PERM_EXEC:  u8 = 1 << 2;


/// Memory permissions for a corresponding address
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Perm(pub u8);

/// A guest Virtual Address
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VAddr(pub usize);


/// Returns the number `num` aligned to `self.alignment`
#[inline(always)]
pub fn align(num: usize) -> usize {
    (num + ALIGNMENT) & !ALIGNMENT
}

/// Memory space of an emulator
pub struct Mmu {
    /// Guest memory address space
    memory: Vec<u8>,

    /// Permissions of the corresponding memory.
    /// This doubles the memory footprint, I am aware
    pub permissions: Vec<Perm>,

    /// Indexes into `dirty_bitmap`
    dirty_indexes: Vec<usize>,

    /// A bitmap tracking dirtied regions in memory
    dirty_bitmap: Vec<u128>,

    /// Base `VAddr` of the next allocation
    alloc_base: VAddr,
}

impl Mmu {
/// Create a new `size` long memory space.
///
/// Allocation base of the virtual memory is set to `0x0`.
/// All memory is aligned to `0xf`.
    pub fn new(size: usize) -> Self {
        // Get the size of the `dirty_bitmap` vector.
        //
        // `usize::BITS` is used here because the vector holds `usize`s which
        // can have a various amount of bits. If the vector type is ever changed
        // to something like `Vec<u64>`, `u64::BITS` should be used instead.
        //
        // `+1` guarantees that we have at least one element tracking
        // a redundant number of regions.
        let dirty_bm_size = size / DIRTY_BLOCK_SIZE / DBE_BITS + 1;
        let aligned_size  = (size + ALIGNMENT) & !ALIGNMENT;

        // Make sure that we have enough memory to track it
        if aligned_size < DIRTY_BLOCK_SIZE {
            panic!("Memory size ({}) can't be larger than the given \
                   DIRTY_BLOCK_SIZE ({}).", size, DIRTY_BLOCK_SIZE);
        }

        Self {
            memory:        vec![0; aligned_size],
            permissions:   vec![Perm(0); aligned_size],
            dirty_indexes: Vec::with_capacity(size / DIRTY_BLOCK_SIZE + 1),
            dirty_bitmap:  vec![0; dirty_bm_size],
            alloc_base:    VAddr(0x0),
        }
    }

    /// Fork the memory state of the current MMU, clearing all dirty bits.
    pub fn fork(&self) -> Self {
        Self {
            memory:        self.memory.clone(),
            permissions:   self.permissions.clone(),
            dirty_indexes: Vec::with_capacity(self.dirty_indexes.capacity()),
            dirty_bitmap:  vec![0; self.dirty_bitmap.len()],
            alloc_base:    self.alloc_base,
        }
    }

    /// Restore the memory state (dirty blocks) of the current MMU to the state
    /// of the `other` MMU.
    pub fn reset(&mut self, other: &Mmu) {
        for &dirty_idx in &self.dirty_indexes {
            let from = dirty_idx * DIRTY_BLOCK_SIZE;
            let to   = (dirty_idx + 1) * DIRTY_BLOCK_SIZE;

            // Reset the bitmap
            self.dirty_bitmap[dirty_idx / DBE_BITS] = 0;

            // Reset the memory
            self.memory[from..to]
                .copy_from_slice(&other.memory[from..to]);

            // Reset the permissions
            self.permissions[from..to]
                .copy_from_slice(&other.permissions[from..to]);
        }
        self.dirty_indexes.clear();
    }

    /// Allocate a region in memory
    pub fn allocate(&mut self, size: usize) -> Option<VAddr> {
        // Update the allocation base
        let cur_base  = VAddr(self.alloc_base.0);
        let next_base = VAddr(cur_base.0.checked_add(align(size))?);

        // Don't allocate OOM
        if next_base.0 > self.memory.len() {
            return None;
        }

        // Mark the memory as writable
        self.set_permissions(cur_base, size, Perm(PERM_WRITE))?;

        self.alloc_base = next_base;
        Some(cur_base)
    }

    /// Set the permissions of a `size` long memory block starting from `addr`
    /// to `perm`
    pub fn set_permissions(&mut self, addr: VAddr,
                           size: usize, perm: Perm) -> Option<()> {
        self.permissions.get_mut(addr.0..addr.0.checked_add(size)?)?
            .iter_mut().for_each(|x| x.0 = perm.0);
        Some(())
    }

    /// Write bytes from `buf` to memory at `addr`.
    /// The resulting bytes are set to be readable (`PERM_READ`)
    pub fn write(&mut self, addr: VAddr, buf: &[u8]) -> Option<()> {
        let from = addr.0;
        let to   = addr.0.checked_add(buf.len())?;

        let perms = self.permissions.get_mut(from..to)?;

        // Check that we can write to memory
        if perms.iter().any(|x| (x.0 & PERM_WRITE) == 0) {
            return None;
        }

        // Write the buffer to memory
        self.memory.get_mut(from..to)?.copy_from_slice(buf);

        // Track the dirty memory
        let dirty_start = addr.0 / DIRTY_BLOCK_SIZE;
        let dirty_end   = to / DIRTY_BLOCK_SIZE;
        for dirty_block in dirty_start..=dirty_end {
            let idx = dirty_start / DBE_BITS;
            let bit = dirty_start % DBE_BITS;

            // Only change the dirty state if the block isn't dirty already
            if self.dirty_bitmap[idx] & (1 << bit) == 0 {
                self.dirty_indexes.push(dirty_block);
                self.dirty_bitmap[idx] |= 1 << bit;
            }
        }

        // RaW: Set the memory to be readable
        perms.iter_mut().for_each(|x| x.0 |= PERM_READ);
        Some(())
    }

    /// Reads bytes from memory at `addr` to `buf`
    pub fn read(&self, addr: VAddr, buf: &mut [u8]) -> Option<()> {
        let from = addr.0;
        let to   = addr.0.checked_add(buf.len())?;

        let perms = self.permissions.get(from..to)?;

        // Check that we can read from the memory
        if perms.iter().any(|x| (x.0 & PERM_READ) == 0) {
            return None;
        }

        // Read the memory
        buf.copy_from_slice(self.memory.get(from..to)?);
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MSG: &[u8] = b"This is some text that is written into the memory.";

    #[test]
    fn byte_perfect_allocation() {
        let mut mem = Mmu::new(DIRTY_BLOCK_SIZE);
        mem.allocate(MSG.len()).unwrap();
    }

    #[test]
    #[should_panic]
    fn out_of_memory() {
        let mut mem = Mmu::new(DIRTY_BLOCK_SIZE);
        mem.allocate(1024).unwrap(); // OK
        mem.allocate(1024).unwrap(); // OK
        mem.allocate(1024).unwrap(); // OK
        mem.allocate(1024).unwrap(); // OK
        mem.allocate(1).unwrap();    // PANIC
    }

    #[test]
    fn read_write() {
        let mut mem = Mmu::new(DIRTY_BLOCK_SIZE);
        let mut buf = [0; MSG.len()];
        mem.allocate(MSG.len()).unwrap();
        mem.write(VAddr(0x0), MSG).unwrap();
        mem.read(VAddr(0x0), &mut buf).unwrap();
        assert!(buf[0..MSG.len()] == *MSG);
    }

    #[test]
    #[should_panic]
    fn read_uninitialized_memory() {
        let mem = Mmu::new(DIRTY_BLOCK_SIZE);
        let mut buf = [0; MSG.len()];
        mem.read(VAddr(0x0), &mut buf).unwrap();
    }

    #[test]
    #[should_panic]
    fn write_unallocated_memory() {
        let mut mem = Mmu::new(DIRTY_BLOCK_SIZE);
        mem.write(VAddr(0x0), MSG).unwrap();
    }

    #[test]
    #[should_panic]
    fn reset_permissions() {
        let mut mem = Mmu::new(DIRTY_BLOCK_SIZE);
        let base = mem.allocate(DIRTY_BLOCK_SIZE).unwrap();
        {
            let mut buf = [0; DIRTY_BLOCK_SIZE];
            let mut new_mem = mem.fork();

            // Here we write to the memory, so we set `PERM_READ`.
            // Consecutive read operations shouldn't panic.
            assert!(new_mem.write(base, &buf).is_some());
            assert!(new_mem.read(base, &mut buf).is_some());

            // When the memory is reset, permissions are reset as well.
            // Since we haven't written to the memory yet, we can't read it.
            new_mem.reset(&mem);
            new_mem.read(base, &mut buf).unwrap();
        }
    }
}
