use core::ffi::c_void;
use core::mem::{offset_of, size_of};

use crate::Guid;

// UEFI 2.11, 4.6.1: the table of ACPI 2.0 or newer; ACPI 1.0 has a GUID of
// its own, which the loader ignores
pub const ACPI_20_TABLE_GUID: Guid = Guid::new(
    0x8868_e871,
    0xe4f1,
    0x11d3,
    [0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81],
);

// UEFI 2.11, 4.6.1
#[repr(C)]
pub(crate) struct ConfigurationTable {
    pub(crate) vendor_guid: Guid,
    pub(crate) vendor_table: *const c_void,
}

const _: () = assert!(offset_of!(ConfigurationTable, vendor_table) == 16);
const _: () = assert!(size_of::<ConfigurationTable>() == 24);
