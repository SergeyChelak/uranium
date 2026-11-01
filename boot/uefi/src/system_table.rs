use core::mem::{offset_of, size_of};
use core::slice;

use address::PhysicalAddress;

use crate::configuration_table::ConfigurationTable;
use crate::firmware_pointer::{FirmwarePointer, UnusedFirmwarePointer};
use crate::{BootServices, Guid, Handle, RuntimeServices, SimpleTextOutputProtocol, TableHeader};

// UEFI 2.11, 4.3.1
const SIGNATURE: u64 = 0x5453_5953_2049_4249;

/// Which of the firmware's tables did not carry the signature of its kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnexpectedSignature {
    /// The pointer passed to the entry point is null or leads elsewhere
    SystemTable,
    /// The system table points to no boot services table, or to another kind
    BootServices,
    /// The system table points to no runtime services table, or to another kind
    RuntimeServices,
}

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
    /// Takes the pointer the firmware passed to the entry point as the system
    /// table, once its signature and those of both services tables match
    ///
    /// # Safety
    ///
    /// `pointer` must be what the firmware passed to `efi_main`, or null, and
    /// boot services must still run for `'a`
    pub unsafe fn from_raw<'a>(
        pointer: *const SystemTable,
    ) -> Result<&'a SystemTable, UnexpectedSignature> {
        // SAFETY: the caller guarantees the pointer is the firmware's system
        // table, valid for `'a`, or null
        let table = unsafe { pointer.as_ref() }.ok_or(UnexpectedSignature::SystemTable)?;
        if !table.header.has_signature(SIGNATURE) {
            return Err(UnexpectedSignature::SystemTable);
        }
        // SAFETY: the system table is valid for `'a`, so the services tables it
        // points to are valid as well
        let boot_services = unsafe { table.boot_services.get() };
        if !boot_services.is_some_and(BootServices::has_valid_signature) {
            return Err(UnexpectedSignature::BootServices);
        }
        // SAFETY: as above
        let runtime_services = unsafe { table.runtime_services.get() };
        if !runtime_services.is_some_and(RuntimeServices::has_valid_signature) {
            return Err(UnexpectedSignature::RuntimeServices);
        }
        Ok(table)
    }

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
    pub fn configuration_table_address(&self, guid: Guid) -> Option<PhysicalAddress> {
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
    use super::{SIGNATURE, SystemTable, UnexpectedSignature};
    use crate::configuration_table::{ACPI_20_TABLE_GUID, ConfigurationTable};
    use crate::firmware_pointer::FirmwarePointer;
    use crate::{
        BootServices, Guid, RuntimeServices, TableHeader, boot_services, runtime_services,
    };
    use address::PhysicalAddress;
    use core::mem::MaybeUninit;
    use core::ptr;

    const BOOT_SERVICES_SIGNATURE: u64 = 0x5652_4553_544f_4f42;
    const RUNTIME_SERVICES_SIGNATURE: u64 = 0x5652_4553_544e_5552;

    /// The three tables of a fake firmware, each under the given signature
    struct Tables {
        boot_services: BootServices,
        runtime_services: RuntimeServices,
    }

    impl Tables {
        fn signed(boot_services: u64, runtime_services: u64) -> Self {
            Self {
                boot_services: boot_services::testing::table_with_header(
                    TableHeader::with_signature(boot_services),
                ),
                runtime_services: runtime_services::testing::table_with_header(
                    TableHeader::with_signature(runtime_services),
                ),
            }
        }

        fn system_table(&self, signature: u64) -> SystemTable {
            let mut table = zeroed_system_table();
            table.header = TableHeader::with_signature(signature);
            table.boot_services = FirmwarePointer::to(&self.boot_services);
            table.runtime_services = FirmwarePointer::to(&self.runtime_services);
            table
        }
    }

    fn from_raw(table: &SystemTable) -> Result<&SystemTable, UnexpectedSignature> {
        // SAFETY: the test keeps `table` and the tables it points to alive
        unsafe { SystemTable::from_raw(table) }
    }

    #[test]
    fn from_raw_accepts_tables_under_their_signatures() {
        let tables = Tables::signed(BOOT_SERVICES_SIGNATURE, RUNTIME_SERVICES_SIGNATURE);
        let table = tables.system_table(SIGNATURE);
        let accepted = from_raw(&table).unwrap();
        assert!(ptr::eq(accepted, &table));
        assert!(ptr::eq(accepted.boot_services(), &tables.boot_services));
    }

    #[test]
    fn from_raw_rejects_a_null_pointer() {
        // SAFETY: a null pointer is allowed and must be refused
        let result = unsafe { SystemTable::from_raw(ptr::null()) };
        assert_eq!(result.err(), Some(UnexpectedSignature::SystemTable));
    }

    #[test]
    fn from_raw_rejects_a_system_table_under_another_signature() {
        let tables = Tables::signed(BOOT_SERVICES_SIGNATURE, RUNTIME_SERVICES_SIGNATURE);
        let table = tables.system_table(SIGNATURE + 1);
        assert_eq!(
            from_raw(&table).err(),
            Some(UnexpectedSignature::SystemTable)
        );
    }

    #[test]
    fn from_raw_rejects_missing_boot_services() {
        let mut table = zeroed_system_table();
        table.header = TableHeader::with_signature(SIGNATURE);
        assert_eq!(
            from_raw(&table).err(),
            Some(UnexpectedSignature::BootServices)
        );
    }

    #[test]
    fn from_raw_rejects_boot_services_under_another_signature() {
        let tables = Tables::signed(0, RUNTIME_SERVICES_SIGNATURE);
        let table = tables.system_table(SIGNATURE);
        assert_eq!(
            from_raw(&table).err(),
            Some(UnexpectedSignature::BootServices)
        );
    }

    #[test]
    fn from_raw_rejects_runtime_services_under_another_signature() {
        let tables = Tables::signed(BOOT_SERVICES_SIGNATURE, 0);
        let table = tables.system_table(SIGNATURE);
        assert_eq!(
            from_raw(&table).err(),
            Some(UnexpectedSignature::RuntimeServices)
        );
    }

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
    fn configuration_table_address_matches_by_guid() {
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
            table.configuration_table_address(ACPI_20_TABLE_GUID),
            Some(PhysicalAddress::new(0x2000))
        );
        assert_eq!(
            table.configuration_table_address(Guid::new(9, 9, 9, [9; 8])),
            None
        );
    }

    #[test]
    fn configuration_tables_is_empty_when_count_is_zero() {
        assert!(zeroed_system_table().configuration_tables().is_empty());
        assert!(system_table_with(&[]).configuration_tables().is_empty());
        assert_eq!(
            system_table_with(&[]).configuration_table_address(ACPI_20_TABLE_GUID),
            None
        );
    }
}
