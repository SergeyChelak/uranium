use core::marker::PhantomData;
use core::slice;

use acpi_tables::{AcpiError, Spcr, TableReader};
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

    fn spcr(&self) -> Result<Spcr, PlatformError> {
        let rsdp = self.rsdp().ok_or(PlatformError::MissingTable)?;
        let tables = FirmwareTables::until_exit(self.system_table);
        Ok(Spcr::find(&tables, rsdp)?)
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

/// The ACPI tables as the firmware left them: identity-mapped, never changed,
/// and readable only while the system table is, so until boot services end
struct FirmwareTables<'a> {
    boot_services: PhantomData<&'a SystemTable>,
}

impl<'a> FirmwareTables<'a> {
    fn until_exit(_system_table: &'a SystemTable) -> Self {
        Self {
            boot_services: PhantomData,
        }
    }
}

impl TableReader for FirmwareTables<'_> {
    fn bytes(&self, address: PhysicalAddress, length: usize) -> Result<&[u8], AcpiError> {
        let source = source(address, length).ok_or(AcpiError::Address)?;
        // SAFETY: the lookup in `acpi-tables`, the only caller of this private
        // type, derives every address from the firmware's chain of tables, which boot
        // services identity-map and leave unchanged; `source` has excluded null
        // and a wrapping range, `length` comes from a 32-bit table length, and
        // the slice borrows `self`, which cannot outlive the system table that
        // `exit_boot_services` consumes
        Ok(unsafe { slice::from_raw_parts(source, length) })
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
