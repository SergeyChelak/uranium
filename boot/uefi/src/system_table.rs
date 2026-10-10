use core::mem::offset_of;

use crate::firmware_pointer::{FirmwarePointer, UnusedFirmwarePointer};
use crate::{BootServices, Handle, RuntimeServices, TableHeader};

/// The table the firmware hands to `efi_main`. A `&SystemTable` may exist
/// only while that pointer is valid: it was created from the pointer the
/// firmware passed, and boot services have not been exited.
// UEFI 2.11, 4.3.1; declared up to BootServices, the last field the loader reads.
#[repr(C)]
pub struct SystemTable {
    header: TableHeader,
    firmware_vendor: UnusedFirmwarePointer,
    firmware_revision: u32,
    console_in_handle: Option<Handle>,
    console_in: UnusedFirmwarePointer,
    console_out_handle: Option<Handle>,
    console_out: UnusedFirmwarePointer,
    standard_error_handle: Option<Handle>,
    standard_error: UnusedFirmwarePointer,
    runtime_services: FirmwarePointer<RuntimeServices>,
    boot_services: FirmwarePointer<BootServices>,
}

impl SystemTable {
    pub fn boot_services(&self) -> &BootServices {
        // SAFETY: a `&SystemTable` exists only while boot services run, so the
        // system table and the boot services table it points to are valid.
        unsafe { self.boot_services.get() }
    }

    pub fn runtime_services(&self) -> &RuntimeServices {
        // SAFETY: a `&SystemTable` exists only while the system table is valid,
        // and the firmware keeps the runtime services table valid as long.
        unsafe { self.runtime_services.get() }
    }
}

const _: () = assert!(offset_of!(SystemTable, console_out) == 64);
const _: () = assert!(offset_of!(SystemTable, runtime_services) == 88);
const _: () = assert!(offset_of!(SystemTable, boot_services) == 96);
