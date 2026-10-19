use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr;

use address::PhysicalAddress;

use crate::firmware_pointer::{UnusedFirmwarePointer, UnusedFunctionPointer};
use crate::memory::{AllocateKind, MemoryMap, MemoryMapKey, MemoryType};
use crate::{Guid, Handle, Protocol, Status, TableHeader};

// UEFI 2.11, 7.2.1
type AllocatePagesFunction =
    unsafe extern "efiapi" fn(AllocateKind, MemoryType, usize, *mut u64) -> Status;

// UEFI 2.11, 7.2.2
type FreePagesFunction = unsafe extern "efiapi" fn(u64, usize) -> Status;

// UEFI 2.11, 7.2.3
type GetMemoryMapFunction =
    unsafe extern "efiapi" fn(*mut usize, *mut c_void, *mut usize, *mut usize, *mut u32) -> Status;

// UEFI 2.11, 7.3.7
type HandleProtocolFunction =
    unsafe extern "efiapi" fn(Handle, *const Guid, *mut *const c_void) -> Status;

// UEFI 2.11, 7.3.16
type LocateProtocolFunction =
    unsafe extern "efiapi" fn(*const Guid, *const c_void, *mut *const c_void) -> Status;

// UEFI 2.11, 7.4.6
type ExitBootServicesFunction = unsafe extern "efiapi" fn(Handle, usize) -> Status;

// UEFI 2.11, 7.5.2
type StallFunction = unsafe extern "efiapi" fn(usize) -> Status;

// UEFI 2.11, 4.4.1; declared up to LocateProtocol, the last field the loader calls
#[repr(C)]
pub struct BootServices {
    header: TableHeader,
    raise_task_priority_level: UnusedFunctionPointer,
    restore_task_priority_level: UnusedFunctionPointer,
    allocate_pages: AllocatePagesFunction,
    free_pages: FreePagesFunction,
    get_memory_map: GetMemoryMapFunction,
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
    handle_protocol: HandleProtocolFunction,
    reserved: UnusedFirmwarePointer,
    register_protocol_notify: UnusedFunctionPointer,
    locate_handle: UnusedFunctionPointer,
    locate_device_path: UnusedFunctionPointer,
    install_configuration_table: UnusedFunctionPointer,
    load_image: UnusedFunctionPointer,
    start_image: UnusedFunctionPointer,
    exit: UnusedFunctionPointer,
    unload_image: UnusedFunctionPointer,
    exit_boot_services: ExitBootServicesFunction,
    get_next_monotonic_count: UnusedFunctionPointer,
    stall: StallFunction,
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
    /// Allocates `count` pages of `memory_type` wherever the firmware chooses
    pub fn allocate_pages(
        &self,
        memory_type: MemoryType,
        count: usize,
    ) -> Result<PhysicalAddress, Status> {
        let mut memory = 0;
        // SAFETY: `self` lives in the boot services table, valid while boot
        // services run; `memory` outlives the call
        unsafe { (self.allocate_pages)(AllocateKind::AnyPages, memory_type, count, &mut memory) }
            .into_result()?;
        Ok(PhysicalAddress::new(memory))
    }

    /// Returns `count` pages at `address`, allocated by `allocate_pages`, to
    /// the firmware
    pub fn free_pages(&self, address: PhysicalAddress, count: usize) -> Result<(), Status> {
        // SAFETY: `self` lives in the boot services table, valid while boot
        // services run
        unsafe { (self.free_pages)(address.as_u64(), count) }.into_result()
    }

    /// The buffer size the next `get_memory_map` needs, with no slack added
    pub fn memory_map_size(&self) -> Result<usize, Status> {
        let mut size = 0;
        let mut key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;
        // SAFETY: a zero size only asks how much is needed, so the null buffer
        // is never written; the other out-pointers are live for the call
        let status = unsafe {
            (self.get_memory_map)(
                &mut size,
                ptr::null_mut(),
                &mut key,
                &mut descriptor_size,
                &mut descriptor_version,
            )
        };
        match status {
            Status::BUFFER_TOO_SMALL => Ok(size),
            other => other.into_result().map(|()| size),
        }
    }

    /// Writes the current map into `buffer`; `BUFFER_TOO_SMALL` when it does
    /// not fit
    pub fn get_memory_map<'a>(&self, buffer: &'a mut [u8]) -> Result<MemoryMap<'a>, Status> {
        let mut size = buffer.len();
        let mut key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;
        // SAFETY: `size` is the length of `buffer`, and every pointer is live
        // for the call
        unsafe {
            (self.get_memory_map)(
                &mut size,
                buffer.as_mut_ptr().cast(),
                &mut key,
                &mut descriptor_size,
                &mut descriptor_version,
            )
        }
        .into_result()?;
        let written = buffer.get(..size).ok_or(Status::DEVICE_ERROR)?;
        MemoryMap::new(
            written,
            descriptor_size,
            descriptor_version,
            MemoryMapKey(key),
        )
    }

    /// Ends boot services for good; `INVALID_PARAMETER` when the map changed
    /// since `key` was taken
    ///
    /// # Safety
    ///
    /// After `Ok` no boot service may be called again, this table included,
    /// and memory the firmware reclaims no longer belongs to the caller
    pub unsafe fn exit_boot_services(
        &self,
        image: Handle,
        key: MemoryMapKey,
    ) -> Result<(), Status> {
        // SAFETY: `self` lives in the boot services table, still valid at the
        // moment of the call
        unsafe { (self.exit_boot_services)(image, key.0) }.into_result()
    }

    /// Busy-waits for at least `microseconds`
    pub fn stall(&self, microseconds: usize) -> Result<(), Status> {
        // SAFETY: `self` lives in the boot services table, valid while boot
        // services run
        unsafe { (self.stall)(microseconds) }.into_result()
    }

    /// The instance of `P` installed on `handle`; `UNSUPPORTED` when the
    /// handle has none
    pub fn handle_protocol<P: Protocol>(&self, handle: Handle) -> Result<&P, Status> {
        let mut interface: *const c_void = ptr::null();
        // SAFETY: `self` lives in the boot services table, valid while boot
        // services run; the GUID and `interface` are live for the call
        unsafe { (self.handle_protocol)(handle, &P::GUID, &mut interface) }.into_result()?;
        // SAFETY: `Protocol` guarantees `P::GUID` names a `P`, and the firmware
        // returned a pointer to a live instance or null
        unsafe { interface.cast::<P>().as_ref() }.ok_or(Status::UNSUPPORTED)
    }

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

const _: () = assert!(offset_of!(BootServices, allocate_pages) == 40);
const _: () = assert!(offset_of!(BootServices, free_pages) == 48);
const _: () = assert!(offset_of!(BootServices, get_memory_map) == 56);
const _: () = assert!(offset_of!(BootServices, handle_protocol) == 152);
const _: () = assert!(offset_of!(BootServices, exit_boot_services) == 232);
const _: () = assert!(offset_of!(BootServices, stall) == 248);
const _: () = assert!(offset_of!(BootServices, locate_protocol) == 320);

#[cfg(test)]
mod tests {
    use super::{
        AllocateKind, AllocatePagesFunction, BootServices, ExitBootServicesFunction,
        FreePagesFunction, GetMemoryMapFunction, HandleProtocolFunction, LocateProtocolFunction,
        StallFunction,
    };
    use crate::{Guid, Handle, MemoryMapKey, MemoryType, Protocol, Status};
    use address::PhysicalAddress;
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

    unsafe extern "efiapi" fn hand_back_the_guid_for_any_handle(
        _handle: Handle,
        guid: *const Guid,
        interface: *mut *const c_void,
    ) -> Status {
        // SAFETY: the test passes a live out-pointer
        unsafe { *interface = guid.cast() };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn support_nothing(
        _handle: Handle,
        _guid: *const Guid,
        interface: *mut *const c_void,
    ) -> Status {
        // SAFETY: the test passes a live out-pointer
        unsafe { *interface = ptr::null() };
        Status::UNSUPPORTED
    }

    unsafe extern "efiapi" fn allocate_three_loader_pages(
        kind: AllocateKind,
        memory_type: MemoryType,
        count: usize,
        memory: *mut u64,
    ) -> Status {
        if kind != AllocateKind::AnyPages || memory_type != MemoryType::LoaderData || count != 3 {
            return Status::INVALID_PARAMETER;
        }
        // SAFETY: the test passes a live out-pointer
        unsafe { *memory = 0x4000 };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn refuse_allocation(
        _kind: AllocateKind,
        _memory_type: MemoryType,
        _count: usize,
        _memory: *mut u64,
    ) -> Status {
        Status::OUT_OF_RESOURCES
    }

    unsafe extern "efiapi" fn free_three_pages_at_0x4000(address: u64, count: usize) -> Status {
        if address == 0x4000 && count == 3 {
            Status::SUCCESS
        } else {
            Status::INVALID_PARAMETER
        }
    }

    unsafe extern "efiapi" fn know_no_pages(_address: u64, _count: usize) -> Status {
        Status::NOT_FOUND
    }

    unsafe extern "efiapi" fn need_four_hundred_bytes(
        size: *mut usize,
        _map: *mut c_void,
        _key: *mut usize,
        _descriptor_size: *mut usize,
        _descriptor_version: *mut u32,
    ) -> Status {
        // SAFETY: the test passes a live size
        unsafe { *size = 400 };
        Status::BUFFER_TOO_SMALL
    }

    unsafe extern "efiapi" fn hand_back_an_empty_map(
        size: *mut usize,
        _map: *mut c_void,
        key: *mut usize,
        descriptor_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> Status {
        // SAFETY: the test passes live out-pointers
        unsafe { *size = 0 };
        // SAFETY: the test passes live out-pointers
        unsafe { *key = 7 };
        // SAFETY: the test passes live out-pointers
        unsafe { *descriptor_size = 40 };
        // SAFETY: the test passes live out-pointers
        unsafe { *descriptor_version = 1 };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn exit_with_key_seven(_image: Handle, key: usize) -> Status {
        if key == 7 {
            Status::SUCCESS
        } else {
            Status::INVALID_PARAMETER
        }
    }

    unsafe extern "efiapi" fn stall_quietly(_microseconds: usize) -> Status {
        Status::SUCCESS
    }

    struct Fakes {
        handle_protocol: HandleProtocolFunction,
        locate_protocol: LocateProtocolFunction,
        allocate_pages: AllocatePagesFunction,
        free_pages: FreePagesFunction,
        get_memory_map: GetMemoryMapFunction,
        exit_boot_services: ExitBootServicesFunction,
        stall: StallFunction,
    }

    impl Fakes {
        /// Fakes that refuse every call; a test overrides the one it exercises
        fn refusing() -> Self {
            Self {
                handle_protocol: support_nothing,
                locate_protocol: find_nothing,
                allocate_pages: refuse_allocation,
                free_pages: know_no_pages,
                get_memory_map: need_four_hundred_bytes,
                exit_boot_services: exit_with_key_seven,
                stall: stall_quietly,
            }
        }
    }

    fn table_with(fakes: Fakes) -> BootServices {
        let mut table = MaybeUninit::<BootServices>::zeroed();
        let place = table.as_mut_ptr();
        // SAFETY: `place` comes from `MaybeUninit`, so the field exists
        let slot = unsafe { &raw mut (*place).handle_protocol };
        // SAFETY: `slot` points inside `table`
        unsafe { slot.write(fakes.handle_protocol) };
        // SAFETY: `place` comes from `MaybeUninit`, so the field exists
        let slot = unsafe { &raw mut (*place).locate_protocol };
        // SAFETY: `slot` points inside `table`
        unsafe { slot.write(fakes.locate_protocol) };
        // SAFETY: `place` comes from `MaybeUninit`, so the field exists
        let slot = unsafe { &raw mut (*place).allocate_pages };
        // SAFETY: `slot` points inside `table`
        unsafe { slot.write(fakes.allocate_pages) };
        // SAFETY: `place` comes from `MaybeUninit`, so the field exists
        let slot = unsafe { &raw mut (*place).free_pages };
        // SAFETY: `slot` points inside `table`
        unsafe { slot.write(fakes.free_pages) };
        // SAFETY: `place` comes from `MaybeUninit`, so the field exists
        let slot = unsafe { &raw mut (*place).get_memory_map };
        // SAFETY: `slot` points inside `table`
        unsafe { slot.write(fakes.get_memory_map) };
        // SAFETY: `place` comes from `MaybeUninit`, so the field exists
        let slot = unsafe { &raw mut (*place).exit_boot_services };
        // SAFETY: `slot` points inside `table`
        unsafe { slot.write(fakes.exit_boot_services) };
        // SAFETY: `place` comes from `MaybeUninit`, so the field exists
        let slot = unsafe { &raw mut (*place).stall };
        // SAFETY: `slot` points inside `table`
        unsafe { slot.write(fakes.stall) };
        // SAFETY: every other field is an integer or a nullable raw pointer,
        // for which all-zero bytes are valid; the seven function pointers were
        // written above
        unsafe { table.assume_init() }
    }

    #[test]
    fn locate_protocol_returns_the_interface_the_firmware_found() {
        let table = table_with(Fakes {
            locate_protocol: hand_back_the_guid,
            ..Fakes::refusing()
        });
        let found: &Probe = table.locate_protocol().unwrap();
        assert_eq!(found.0, Probe::GUID);
    }

    #[test]
    fn locate_protocol_reports_a_missing_protocol() {
        let table = table_with(Fakes::refusing());
        let result = table.locate_protocol::<Probe>();
        assert_eq!(result.err(), Some(Status::NOT_FOUND));
    }

    #[test]
    fn handle_protocol_returns_the_interface() {
        let table = table_with(Fakes {
            handle_protocol: hand_back_the_guid_for_any_handle,
            ..Fakes::refusing()
        });
        let found: &Probe = table.handle_protocol(Handle::dangling()).unwrap();
        assert_eq!(found.0, Probe::GUID);
    }

    #[test]
    fn handle_protocol_maps_unsupported() {
        let table = table_with(Fakes::refusing());
        let result = table.handle_protocol::<Probe>(Handle::dangling());
        assert_eq!(result.err(), Some(Status::UNSUPPORTED));
    }

    #[test]
    fn allocate_pages_returns_the_address() {
        let table = table_with(Fakes {
            allocate_pages: allocate_three_loader_pages,
            ..Fakes::refusing()
        });
        assert_eq!(
            table.allocate_pages(MemoryType::LoaderData, 3),
            Ok(PhysicalAddress::new(0x4000))
        );
        assert_eq!(
            table.allocate_pages(MemoryType::LoaderData, 4),
            Err(Status::INVALID_PARAMETER)
        );
    }

    #[test]
    fn free_pages_hands_the_pages_back() {
        let table = table_with(Fakes {
            free_pages: free_three_pages_at_0x4000,
            ..Fakes::refusing()
        });
        assert_eq!(table.free_pages(PhysicalAddress::new(0x4000), 3), Ok(()));
        assert_eq!(
            table.free_pages(PhysicalAddress::new(0x4000), 4),
            Err(Status::INVALID_PARAMETER)
        );
    }

    #[test]
    fn free_pages_reports_pages_the_firmware_never_allocated() {
        let table = table_with(Fakes::refusing());
        assert_eq!(
            table.free_pages(PhysicalAddress::new(0x4000), 3),
            Err(Status::NOT_FOUND)
        );
    }

    #[test]
    fn memory_map_size_reads_buffer_too_small() {
        let table = table_with(Fakes::refusing());
        assert_eq!(table.memory_map_size(), Ok(400));
    }

    #[test]
    fn get_memory_map_hands_the_key_to_exit_boot_services() {
        let table = table_with(Fakes {
            get_memory_map: hand_back_an_empty_map,
            ..Fakes::refusing()
        });
        let mut buffer = [0; 80];
        let map = table.get_memory_map(&mut buffer).unwrap();
        assert_eq!(map.descriptors().count(), 0);
        assert_eq!(map.descriptor_size(), 40);
        // SAFETY: the fake firmware keeps serving calls after a successful exit
        let exited = unsafe { table.exit_boot_services(Handle::dangling(), map.key()) };
        assert_eq!(exited, Ok(()));
        // SAFETY: as above
        let stale = unsafe { table.exit_boot_services(Handle::dangling(), MemoryMapKey(8)) };
        assert_eq!(stale, Err(Status::INVALID_PARAMETER));
    }

    #[test]
    fn stall_reports_the_firmware_status() {
        let table = table_with(Fakes::refusing());
        assert_eq!(table.stall(5), Ok(()));
    }
}
