use core::slice;

use address::VirtualAddress;
use firmware::{Allocation, Memory, MemoryError};
use uefi::{BootServices, MemoryType, PAGE_SIZE, Status};

pub struct UefiMemory<'a> {
    boot_services: &'a BootServices,
}

impl<'a> UefiMemory<'a> {
    pub(crate) fn new(boot_services: &'a BootServices) -> Self {
        Self { boot_services }
    }
}

impl Memory for UefiMemory<'_> {
    fn allocate(&self, page_count: u64) -> Result<Allocation<'_>, MemoryError> {
        let count = usize::try_from(page_count).map_err(|_| MemoryError::OutOfMemory)?;
        let length = count
            .checked_mul(PAGE_SIZE)
            .ok_or(MemoryError::OutOfMemory)?;
        let base = self
            .boot_services
            .allocate_pages(MemoryType::LoaderData, count)
            .map_err(allocate_error)?;
        let start = VirtualAddress::new(base.as_u64()).as_mut_pointer::<u8>();
        // SAFETY: boot services identity-map the whole memory map, so `start`
        // addresses the `length` bytes the firmware has just handed to this
        // caller alone, and the loader never frees them
        let bytes = unsafe { slice::from_raw_parts_mut(start, length) };
        Ok(Allocation { base, bytes })
    }

    fn memory_map_size(&self) -> Result<usize, MemoryError> {
        self.boot_services
            .memory_map_size()
            .map_err(|_| MemoryError::Device)
    }
}

fn allocate_error(status: Status) -> MemoryError {
    match status {
        Status::OUT_OF_RESOURCES => MemoryError::OutOfMemory,
        _ => MemoryError::Device,
    }
}

#[cfg(test)]
mod tests {
    use super::allocate_error;
    use firmware::MemoryError;
    use uefi::Status;

    #[test]
    fn out_of_resources_means_out_of_memory() {
        assert_eq!(
            allocate_error(Status::OUT_OF_RESOURCES),
            MemoryError::OutOfMemory
        );
    }

    #[test]
    fn every_other_status_is_a_device_failure() {
        assert_eq!(
            allocate_error(Status::INVALID_PARAMETER),
            MemoryError::Device
        );
        assert_eq!(allocate_error(Status::NOT_FOUND), MemoryError::Device);
    }
}
