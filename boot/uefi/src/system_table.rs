use core::mem::offset_of;

use crate::firmware_pointer::{FirmwarePointer, UnusedFirmwarePointer};
use crate::{BootServices, Handle, RuntimeServices, SimpleTextOutputProtocol, TableHeader};

/// The table the firmware hands to `efi_main`. A `&SystemTable` may exist
/// only while that pointer is valid: it was created from the pointer the
/// firmware passed, and boot services have not been exited
// UEFI 2.11, 4.3.1; declared up to BootServices, the last field the loader reads
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
}

const _: () = assert!(offset_of!(SystemTable, console_out) == 64);
const _: () = assert!(offset_of!(SystemTable, runtime_services) == 88);
const _: () = assert!(offset_of!(SystemTable, boot_services) == 96);
