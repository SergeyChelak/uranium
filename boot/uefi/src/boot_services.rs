use core::mem::offset_of;

use crate::TableHeader;
use crate::firmware_pointer::{UnusedFirmwarePointer, UnusedFunctionPointer};

// UEFI 2.11, 4.4.1; declared up to LocateProtocol, the last field the loader calls.
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
    locate_protocol: UnusedFunctionPointer,
}

const _: () = assert!(offset_of!(BootServices, locate_protocol) == 320);
