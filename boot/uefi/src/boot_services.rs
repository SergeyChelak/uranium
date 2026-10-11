use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr;

use crate::firmware_pointer::{UnusedFirmwarePointer, UnusedFunctionPointer};
use crate::{Guid, Protocol, Status, TableHeader};

// UEFI 2.11, 7.3.16
type LocateProtocolFunction =
    unsafe extern "efiapi" fn(*const Guid, *const c_void, *mut *const c_void) -> Status;

// UEFI 2.11, 4.4.1; declared up to LocateProtocol, the last field the loader calls
#[repr(C)]
pub struct BootServices {
    header: TableHeader,
    raise_task_priority_level: UnusedFunctionPointer,
    restore_task_priority_level: UnusedFunctionPointer,
    allocate_pages: UnusedFunctionPointer,
    free_pages: UnusedFunctionPointer,
    get_memory_map: UnusedFunctionPointer,
    allocate_pool: UnusedFunctionPointer,
    free_pool: UnusedFunctionPointer,
    create_event: UnusedFunctionPointer,
    set_timer: UnusedFunctionPointer,
    wait_for_event: UnusedFunctionPointer,
    signal_event: UnusedFunctionPointer,
    close_event: UnusedFunctionPointer,
    check_event: UnusedFunctionPointer,
    install_protocol_interface: UnusedFunctionPointer,
    reinstall_protocol_interface: UnusedFunctionPointer,
    uninstall_protocol_interface: UnusedFunctionPointer,
    handle_protocol: UnusedFunctionPointer,
    reserved: UnusedFirmwarePointer,
    register_protocol_notify: UnusedFunctionPointer,
    locate_handle: UnusedFunctionPointer,
    locate_device_path: UnusedFunctionPointer,
    install_configuration_table: UnusedFunctionPointer,
    load_image: UnusedFunctionPointer,
    start_image: UnusedFunctionPointer,
    exit: UnusedFunctionPointer,
    unload_image: UnusedFunctionPointer,
    exit_boot_services: UnusedFunctionPointer,
    get_next_monotonic_count: UnusedFunctionPointer,
    stall: UnusedFunctionPointer,
    set_watchdog_timer: UnusedFunctionPointer,
    connect_controller: UnusedFunctionPointer,
    disconnect_controller: UnusedFunctionPointer,
    open_protocol: UnusedFunctionPointer,
    close_protocol: UnusedFunctionPointer,
    open_protocol_information: UnusedFunctionPointer,
    protocols_per_handle: UnusedFunctionPointer,
    locate_handle_buffer: UnusedFunctionPointer,
    locate_protocol: LocateProtocolFunction,
}

impl BootServices {
    /// Finds the first instance of `P` the firmware offers; `NOT_FOUND`
    /// when there is none
    pub fn locate_protocol<P: Protocol>(&self) -> Result<&P, Status> {
        let mut interface: *const c_void = ptr::null();
        // SAFETY: `self` lives in the boot services table, valid while boot
        // services run; the GUID and `interface` are live for the call
        unsafe { (self.locate_protocol)(&P::GUID, ptr::null(), &mut interface) }.into_result()?;
        // SAFETY: `Protocol` guarantees `P::GUID` names a `P`, and the firmware
        // returned a pointer to a live instance or null
        unsafe { interface.cast::<P>().as_ref() }.ok_or(Status::NOT_FOUND)
    }
}

const _: () = assert!(offset_of!(BootServices, locate_protocol) == 320);

#[cfg(test)]
mod tests {
    use super::{BootServices, LocateProtocolFunction};
    use crate::{Guid, Protocol, Status};
    use core::ffi::c_void;
    use core::mem::MaybeUninit;
    use core::ptr;

    #[repr(transparent)]
    struct Probe(Guid);

    // SAFETY: the fake firmware below hands back the pointer to the GUID it was
    // asked for, and `Probe` is transparent over `Guid`
    unsafe impl Protocol for Probe {
        const GUID: Guid = Guid::new(1, 2, 3, [4; 8]);
    }

    unsafe extern "efiapi" fn hand_back_the_guid(
        guid: *const Guid,
        _registration: *const c_void,
        interface: *mut *const c_void,
    ) -> Status {
        // SAFETY: the test passes a live out-pointer
        unsafe { *interface = guid.cast() };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn find_nothing(
        _guid: *const Guid,
        _registration: *const c_void,
        interface: *mut *const c_void,
    ) -> Status {
        // SAFETY: the test passes a live out-pointer
        unsafe { *interface = ptr::null() };
        Status::NOT_FOUND
    }

    fn table_with(locate_protocol: LocateProtocolFunction) -> BootServices {
        let mut table = MaybeUninit::<BootServices>::zeroed();
        // SAFETY: the pointer comes from `MaybeUninit`, so the place exists
        let slot = unsafe { &raw mut (*table.as_mut_ptr()).locate_protocol };
        // SAFETY: `slot` points inside `table`
        unsafe { slot.write(locate_protocol) };
        // SAFETY: every other field is an integer or a nullable raw pointer,
        // for which all-zero bytes are valid; the one function pointer was
        // written above
        unsafe { table.assume_init() }
    }

    #[test]
    fn locate_protocol_returns_the_interface_the_firmware_found() {
        let table = table_with(hand_back_the_guid);
        let found: &Probe = table.locate_protocol().unwrap();
        assert_eq!(found.0, Probe::GUID);
    }

    #[test]
    fn locate_protocol_reports_a_missing_protocol() {
        let table = table_with(find_nothing);
        let result = table.locate_protocol::<Probe>();
        assert_eq!(result.err(), Some(Status::NOT_FOUND));
    }
}
