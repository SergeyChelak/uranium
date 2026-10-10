use core::mem::offset_of;

use crate::TableHeader;
use crate::firmware_pointer::UnusedFunctionPointer;

// UEFI 2.11, 4.5.1; declared up to ResetSystem, the last field the loader calls.
#[repr(C)]
pub struct RuntimeServices {
    header: TableHeader,
    get_time: UnusedFunctionPointer,
    set_time: UnusedFunctionPointer,
    get_wakeup_time: UnusedFunctionPointer,
    set_wakeup_time: UnusedFunctionPointer,
    set_virtual_address_map: UnusedFunctionPointer,
    convert_pointer: UnusedFunctionPointer,
    get_variable: UnusedFunctionPointer,
    get_next_variable_name: UnusedFunctionPointer,
    set_variable: UnusedFunctionPointer,
    get_next_high_monotonic_count: UnusedFunctionPointer,
    reset_system: UnusedFunctionPointer,
}

const _: () = assert!(offset_of!(RuntimeServices, reset_system) == 104);
