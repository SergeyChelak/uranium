use core::mem;

use address::PhysicalAddress;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    /// The firmware has no run of free pages that long
    OutOfMemory,
    /// The firmware reported a failure
    Device,
}

/// Pages the firmware gave the loader: their physical base and a view of the
/// same bytes through the firmware's identity mapping, which lasts until the
/// handoff. Dropping the value hands the pages back to the firmware, so every
/// error path frees what it allocated; `leak` keeps them for the kernel
pub struct Allocation<'a, M: Memory> {
    base: PhysicalAddress,
    bytes: &'static mut [u8],
    memory: &'a M,
}

impl<'a, M: Memory> Allocation<'a, M> {
    pub fn new(base: PhysicalAddress, bytes: &'static mut [u8], memory: &'a M) -> Self {
        Self {
            base,
            bytes,
            memory,
        }
    }

    pub fn base(&self) -> PhysicalAddress {
        self.base
    }

    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes
    }

    /// Keeps the pages for the kernel: nothing frees them from now on. The
    /// bytes outlive boot services; read `base` before leaking
    pub fn leak(mut self) -> &'static mut [u8] {
        mem::take(&mut self.bytes)
    }

    /// Hands the pages back to the firmware now, reporting its answer
    pub fn free(mut self) -> Result<(), MemoryError> {
        let bytes = mem::take(&mut self.bytes);
        self.memory.free(self.base, bytes)
    }
}

impl<M: Memory> Drop for Allocation<'_, M> {
    fn drop(&mut self) {
        if self.bytes.is_empty() {
            return;
        }
        let bytes = mem::take(&mut self.bytes);
        // a refusal cannot be reported from here, and means the firmware
        // lost track of pages it handed out itself
        let _ = self.memory.free(self.base, bytes);
    }
}

pub trait Memory {
    /// `page_count` contiguous pages of 4 KiB, wherever the firmware chooses
    fn allocate(&self, page_count: u64) -> Result<Allocation<'_, Self>, MemoryError>
    where
        Self: Sized;

    /// Returns pages that came from `allocate` to the firmware: their base
    /// and the whole view of them
    fn free(&self, base: PhysicalAddress, bytes: &'static mut [u8]) -> Result<(), MemoryError>;

    /// The buffer size the firmware's memory map needs right now, with no
    /// slack added
    fn memory_map_size(&self) -> Result<usize, MemoryError>;
}

/// What a region of physical memory may be used for: the firmware's picture
/// reduced to what the kernel must know
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    /// Free for the kernel once the loader is gone
    Usable,
    /// Never to be touched
    Reserved,
    /// ACPI tables; free once the kernel has read them
    AcpiReclaimable,
    /// ACPI non-volatile storage; preserved across sleep states
    AcpiNonVolatile,
    /// Device registers mapped into memory
    MemoryMappedIo,
    /// Code and data of the firmware's runtime services; preserved
    UefiRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    pub start: PhysicalAddress,
    pub page_count: u64,
    pub kind: MemoryKind,
}

/// What leaving boot services leaves behind: how much of each buffer was
/// filled and how the raw map is laid out
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMapSummary {
    pub region_count: usize,
    pub raw_length: usize,
    pub raw_descriptor_size: usize,
    pub raw_descriptor_version: u32,
}

#[cfg(test)]
mod tests {
    use super::{Allocation, Memory, MemoryError};
    use address::PhysicalAddress;
    use core::cell::Cell;
    use std::boxed::Box;
    use std::vec;

    /// Counts the pages it has handed out and not yet taken back
    struct Ledger {
        outstanding: Cell<usize>,
        refuses: bool,
    }

    impl Ledger {
        fn new() -> Self {
            Self {
                outstanding: Cell::new(0),
                refuses: false,
            }
        }
    }

    impl Memory for Ledger {
        fn allocate(&self, page_count: u64) -> Result<Allocation<'_, Self>, MemoryError> {
            let length = usize::try_from(page_count).unwrap() * 4096;
            let bytes = Box::leak(vec![0; length].into_boxed_slice());
            self.outstanding.set(self.outstanding.get() + 1);
            Ok(Allocation::new(PhysicalAddress::new(0x1000), bytes, self))
        }

        fn free(
            &self,
            _base: PhysicalAddress,
            _bytes: &'static mut [u8],
        ) -> Result<(), MemoryError> {
            if self.refuses {
                return Err(MemoryError::Device);
            }
            self.outstanding.set(self.outstanding.get() - 1);
            Ok(())
        }

        fn memory_map_size(&self) -> Result<usize, MemoryError> {
            Ok(0)
        }
    }

    #[test]
    fn dropping_an_allocation_hands_the_pages_back() {
        let ledger = Ledger::new();
        {
            let mut allocation = ledger.allocate(2).unwrap();
            allocation.bytes_mut()[0] = 7;
            assert_eq!(allocation.bytes().len(), 8192);
            assert_eq!(allocation.base(), PhysicalAddress::new(0x1000));
            assert_eq!(ledger.outstanding.get(), 1);
        }
        assert_eq!(ledger.outstanding.get(), 0);
    }

    #[test]
    fn leaked_pages_stay_allocated() {
        let ledger = Ledger::new();
        let bytes = ledger.allocate(1).unwrap().leak();
        assert_eq!(bytes.len(), 4096);
        assert_eq!(ledger.outstanding.get(), 1);
    }

    #[test]
    fn free_hands_the_pages_back_once() {
        let ledger = Ledger::new();
        let allocation = ledger.allocate(1).unwrap();
        assert_eq!(allocation.free(), Ok(()));
        assert_eq!(ledger.outstanding.get(), 0);
    }

    #[test]
    fn free_reports_the_refusal() {
        let ledger = Ledger {
            refuses: true,
            ..Ledger::new()
        };
        let allocation = ledger.allocate(1).unwrap();
        assert_eq!(allocation.free(), Err(MemoryError::Device));
    }

    #[test]
    fn an_error_path_frees_everything_it_allocated() {
        fn fail_after_two(ledger: &Ledger) -> Result<(), MemoryError> {
            let _first = ledger.allocate(1)?;
            let _second = ledger.allocate(1)?;
            Err(MemoryError::Device)
        }
        let ledger = Ledger::new();
        assert_eq!(fail_after_two(&ledger), Err(MemoryError::Device));
        assert_eq!(ledger.outstanding.get(), 0);
    }
}
