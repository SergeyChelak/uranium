use core::ffi::c_void;
use core::hint::spin_loop;
use core::mem::offset_of;
use core::ptr;

use crate::firmware_pointer::UnusedFunctionPointer;
use crate::{Status, TableHeader};

// UEFI 2.11, 8.5.1.1
type ResetSystemFunction = unsafe extern "efiapi" fn(ResetKind, Status, usize, *const c_void);

// UEFI 2.11, 8.5.1.1, EFI_RESET_TYPE: the specification fixes only the order
// of the variants; the values follow the C rules for an enum without initializers
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum ResetKind {
    Cold = 0,
    Warm = 1,
    Shutdown = 2,
    PlatformSpecific = 3,
}

// UEFI 2.11, 4.5.1
const SIGNATURE: u64 = 0x5652_4553_544e_5552;

// UEFI 2.11, 4.5.1; declared up to ResetSystem, the last field the loader calls
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
    reset_system: ResetSystemFunction,
}

impl RuntimeServices {
    pub(crate) fn has_valid_signature(&self) -> bool {
        self.header.has_signature(SIGNATURE)
    }

    /// If the firmware returns anyway, the processor is parked here
    pub fn reset_system(&self, kind: ResetKind, status: Status) -> ! {
        // SAFETY: `self` lives in the runtime services table, valid as long as
        // the system table; no reset data is passed, so the size is zero and
        // the pointer null
        unsafe { (self.reset_system)(kind, status, 0, ptr::null()) };
        loop {
            spin_loop();
        }
    }
}

const _: () = assert!(offset_of!(RuntimeServices, reset_system) == 104);

#[cfg(test)]
pub(crate) mod testing {
    use super::{ResetKind, RuntimeServices};
    use crate::{Status, TableHeader};
    use core::ffi::c_void;
    use core::mem::MaybeUninit;

    unsafe extern "efiapi" fn never_reset(
        _kind: ResetKind,
        _status: Status,
        _size: usize,
        _data: *const c_void,
    ) {
    }

    pub(crate) fn table_with_header(header: TableHeader) -> RuntimeServices {
        let mut table = MaybeUninit::<RuntimeServices>::zeroed();
        let place = table.as_mut_ptr();
        // SAFETY: `place` comes from `MaybeUninit`, so the field exists
        let slot = unsafe { &raw mut (*place).header };
        // SAFETY: `slot` points inside `table`
        unsafe { slot.write(header) };
        // SAFETY: `place` comes from `MaybeUninit`, so the field exists
        let slot = unsafe { &raw mut (*place).reset_system };
        // SAFETY: `slot` points inside `table`
        unsafe { slot.write(never_reset) };
        // SAFETY: every other field is a nullable raw pointer, for which
        // all-zero bytes are valid; the header and the function pointer were
        // written above
        unsafe { table.assume_init() }
    }
}

#[cfg(test)]
mod tests {
    use super::ResetKind;
    use core::mem::size_of;

    #[test]
    fn reset_kind_is_a_four_byte_enumeration() {
        assert_eq!(size_of::<ResetKind>(), 4);
    }

    #[test]
    fn reset_kinds_follow_the_order_of_the_specification() {
        assert_eq!(ResetKind::Cold as u32, 0);
        assert_eq!(ResetKind::Warm as u32, 1);
        assert_eq!(ResetKind::Shutdown as u32, 2);
        assert_eq!(ResetKind::PlatformSpecific as u32, 3);
    }
}
