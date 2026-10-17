use core::ptr;

use address::{PhysicalAddress, VirtualAddress};
use firmware::{Platform, PlatformError};
use uefi::{ACPI_20_TABLE_GUID, Handle, LoadedImageProtocol, SystemTable, ucs2};

pub struct UefiPlatform<'a> {
    image: Handle,
    system_table: &'a SystemTable,
}

impl<'a> UefiPlatform<'a> {
    pub(crate) fn new(image: Handle, system_table: &'a SystemTable) -> Self {
        Self {
            image,
            system_table,
        }
    }
}

impl Platform for UefiPlatform<'_> {
    fn system_table(&self) -> PhysicalAddress {
        PhysicalAddress::new(VirtualAddress::from_pointer(self.system_table).as_u64())
    }

    fn rsdp(&self) -> Option<PhysicalAddress> {
        self.system_table
            .find_configuration_table(ACPI_20_TABLE_GUID)
    }

    fn read(&self, address: PhysicalAddress, buffer: &mut [u8]) -> Result<(), PlatformError> {
        let source = source(address, buffer.len()).ok_or(PlatformError::Unreadable)?;
        // SAFETY: `Platform::read` takes addresses the firmware provided, which
        // boot services identity-map; `buffer` is the loader's own memory, so
        // the two ranges are disjoint
        unsafe { ptr::copy_nonoverlapping(source, buffer.as_mut_ptr(), buffer.len()) };
        Ok(())
    }

    fn load_options(&self, buffer: &mut [u8]) -> Result<usize, PlatformError> {
        let image = self
            .system_table
            .boot_services()
            .handle_protocol::<LoadedImageProtocol>(self.image)
            .map_err(|_| PlatformError::Unreadable)?;
        match image.load_options() {
            Some(options) => {
                ucs2::decode_terminated(options, buffer).ok_or(PlatformError::Unreadable)
            }
            None => Ok(0),
        }
    }
}

fn source(address: PhysicalAddress, length: usize) -> Option<*const u8> {
    if address.as_u64() == 0 {
        return None;
    }
    address.checked_add(u64::try_from(length).ok()?)?;
    Some(VirtualAddress::new(address.as_u64()).as_pointer())
}

#[cfg(test)]
mod tests {
    use super::source;
    use address::PhysicalAddress;

    #[test]
    fn source_refuses_the_null_address() {
        assert_eq!(source(PhysicalAddress::new(0), 16), None);
    }

    #[test]
    fn source_refuses_a_range_that_wraps_around() {
        assert_eq!(source(PhysicalAddress::new(u64::MAX - 7), 16), None);
        assert!(source(PhysicalAddress::new(u64::MAX - 16), 16).is_some());
    }

    #[test]
    fn source_is_the_identity_mapping() {
        let pointer = source(PhysicalAddress::new(0x1000), 16).unwrap();
        assert_eq!(pointer.addr(), 0x1000);
    }
}
