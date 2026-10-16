use core::mem::{offset_of, size_of};
use core::slice;

use address::PhysicalAddress;

use crate::configuration_table::ConfigurationTable;
use crate::firmware_pointer::{FirmwarePointer, UnusedFirmwarePointer};
use crate::{BootServices, Guid, Handle, RuntimeServices, SimpleTextOutputProtocol, TableHeader};

/// The table the firmware hands to `efi_main`. A `&SystemTable` may exist
/// only while that pointer is valid: it was created from the pointer the
/// firmware passed, and boot services have not been exited
// UEFI 2.11, 4.3.1
#[repr(C)]
pub struct SystemTable {
    header: TableHeader,
    firmware_vendor: UnusedFirmwarePointer,
    firmware_revision: u32,
    console_in_handle: Option<Handle>,
    console_in: UnusedFirmwarePointer,
    console_out_handle: Option<Handle>,
    console_out: FirmwarePointer<SimpleTextOutputProtocol>,
    standard_error_handle: Option<Handle>,
    standard_error: UnusedFirmwarePointer,
    runtime_services: FirmwarePointer<RuntimeServices>,
    boot_services: FirmwarePointer<BootServices>,
    number_of_table_entries: usize,
    configuration_table: *const ConfigurationTable,
}

impl SystemTable {
    /// The active console, or `None` when the firmware has none
    pub fn console_out(&self) -> Option<&SimpleTextOutputProtocol> {
        // SAFETY: a `&SystemTable` exists only while the system table is valid,
        // so the console protocol it points to is valid as well
        unsafe { self.console_out.get() }
    }

    pub fn boot_services(&self) -> &BootServices {
        // SAFETY: a `&SystemTable` exists only while boot services run, so the
        // system table and the boot services table it points to are valid
        unsafe { self.boot_services.get() }
            .expect("the firmware provides the boot services table while boot services run")
    }

    pub fn runtime_services(&self) -> &RuntimeServices {
        // SAFETY: a `&SystemTable` exists only while the system table is valid,
        // and the firmware keeps the runtime services table valid as long
        unsafe { self.runtime_services.get() }
            .expect("the firmware provides the runtime services table")
    }

    /// The physical address of the table the firmware installed under `guid`;
    /// these addresses are physical and are never fixed up
    pub fn find_configuration_table(&self, guid: Guid) -> Option<PhysicalAddress> {
        self.configuration_tables()
            .iter()
            .find(|entry| entry.vendor_guid == guid)
            .map(|entry| PhysicalAddress::new(entry.vendor_table.addr() as u64))
    }

    fn configuration_tables(&self) -> &[ConfigurationTable] {
        if self.configuration_table.is_null() || self.number_of_table_entries == 0 {
            return &[];
        }
        // SAFETY: a `&SystemTable` exists only while the system table is valid,
        // and the firmware keeps `number_of_table_entries` entries behind
        // `configuration_table` as long
        unsafe { slice::from_raw_parts(self.configuration_table, self.number_of_table_entries) }
    }
}

const _: () = assert!(offset_of!(SystemTable, console_out) == 64);
const _: () = assert!(offset_of!(SystemTable, runtime_services) == 88);
const _: () = assert!(offset_of!(SystemTable, boot_services) == 96);
const _: () = assert!(offset_of!(SystemTable, number_of_table_entries) == 104);
const _: () = assert!(offset_of!(SystemTable, configuration_table) == 112);
const _: () = assert!(size_of::<SystemTable>() == 120);

#[cfg(test)]
mod tests {
    use super::SystemTable;
    use crate::Guid;
    use crate::configuration_table::{ACPI_20_TABLE_GUID, ConfigurationTable};
    use address::PhysicalAddress;
    use core::mem::MaybeUninit;
    use core::ptr;

    fn zeroed_system_table() -> SystemTable {
        // SAFETY: every field is an integer or a nullable pointer, for which
        // all-zero bytes are valid
        unsafe { MaybeUninit::<SystemTable>::zeroed().assume_init() }
    }

    fn system_table_with(entries: &[ConfigurationTable]) -> SystemTable {
        let mut table = zeroed_system_table();
        table.number_of_table_entries = entries.len();
        table.configuration_table = entries.as_ptr();
        table
    }

    #[test]
    fn find_configuration_table_matches_by_guid() {
        let entries = [
            ConfigurationTable {
                vendor_guid: Guid::new(1, 2, 3, [4; 8]),
                vendor_table: ptr::without_provenance(0x1000),
            },
            ConfigurationTable {
                vendor_guid: ACPI_20_TABLE_GUID,
                vendor_table: ptr::without_provenance(0x2000),
            },
        ];
        let table = system_table_with(&entries);
        assert_eq!(
            table.find_configuration_table(ACPI_20_TABLE_GUID),
            Some(PhysicalAddress::new(0x2000))
        );
        assert_eq!(
            table.find_configuration_table(Guid::new(9, 9, 9, [9; 8])),
            None
        );
    }

    #[test]
    fn configuration_tables_is_empty_when_count_is_zero() {
        assert!(zeroed_system_table().configuration_tables().is_empty());
        assert!(system_table_with(&[]).configuration_tables().is_empty());
        assert_eq!(
            system_table_with(&[]).find_configuration_table(ACPI_20_TABLE_GUID),
            None
        );
    }
}
