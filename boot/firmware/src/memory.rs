use core::mem;

use address::PhysicalAddress;

use crate::FirmwareError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    OutOfMemory,
    Firmware(FirmwareError),
}

impl From<FirmwareError> for MemoryError {
    fn from(error: FirmwareError) -> Self {
        Self::Firmware(error)
    }
}

/// Dropping the value hands the pages back to the firmware; `leak` keeps
/// them for the kernel
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

    pub fn leak(mut self) -> LeakedMemory {
        LeakedMemory {
            base: self.base,
            bytes: mem::take(&mut self.bytes),
        }
    }

    /// Like dropping, but reports the firmware's answer
    pub fn free(mut self) -> Result<(), MemoryError> {
        let bytes = mem::take(&mut self.bytes);
        self.memory.free(self.base, bytes)
    }
}

/// Memory the firmware will never reclaim: the kernel finds it by `base`,
/// and the loader still reaches it through `bytes` until the handoff
pub struct LeakedMemory {
    pub base: PhysicalAddress,
    pub bytes: &'static mut [u8],
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

    /// Takes back exactly what `allocate` gave
    fn free(&self, base: PhysicalAddress, bytes: &'static mut [u8]) -> Result<(), MemoryError>;

    /// The buffer size the firmware's memory map needs right now, with no
    /// slack added
    fn memory_map_size(&self) -> Result<usize, MemoryError>;
}

/// What a region of physical memory may be used for, as the firmware's map
/// tells it; the kinds the loader assigns itself (its own allocations) exist
/// only in the boot protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    Usable,
    Reserved,
    AcpiReclaimable,
    AcpiNonVolatile,
    MemoryMappedIo,
    UefiRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    pub base: PhysicalAddress,
    pub page_count: u64,
    pub kind: MemoryKind,
}

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
    use crate::{FirmwareError, FirmwareStatus};
    use address::PhysicalAddress;
    use core::cell::Cell;
    use std::boxed::Box;
    use std::vec;

    const REFUSAL: MemoryError =
        MemoryError::Firmware(FirmwareError::Status(FirmwareStatus::new(7)));

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
                return Err(REFUSAL);
            }
            self.outstanding.set(self.outstanding.get() - 1);
            Ok(())
        }

        fn memory_map_size(&self) -> Result<usize, MemoryError> {
            Ok(0)
        }
    }

    #[test]
    fn a_firmware_fault_is_a_memory_error() {
        assert_eq!(
            MemoryError::from(FirmwareError::Status(FirmwareStatus::new(7))),
            REFUSAL
        );
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
    fn leaked_pages_stay_allocated_and_keep_their_base() {
        let ledger = Ledger::new();
        let memory = ledger.allocate(1).unwrap().leak();
        assert_eq!(memory.base, PhysicalAddress::new(0x1000));
        assert_eq!(memory.bytes.len(), 4096);
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
        assert_eq!(allocation.free(), Err(REFUSAL));
    }

    #[test]
    fn an_error_path_frees_everything_it_allocated() {
        fn fail_after_two(ledger: &Ledger) -> Result<(), MemoryError> {
            let _first = ledger.allocate(1)?;
            let _second = ledger.allocate(1)?;
            Err(REFUSAL)
        }
        let ledger = Ledger::new();
        assert_eq!(fail_after_two(&ledger), Err(REFUSAL));
        assert_eq!(ledger.outstanding.get(), 0);
    }
}
