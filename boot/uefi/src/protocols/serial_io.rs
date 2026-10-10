use core::ffi::c_void;
use core::mem::offset_of;

use crate::firmware_pointer::UnusedFunctionPointer;
use crate::{Guid, Protocol, Status};

// UEFI 2.11, 12.8.3.5
type WriteFunction =
    unsafe extern "efiapi" fn(*const SerialIoProtocol, *mut usize, *const c_void) -> Status;

// UEFI 2.11, 12.8.1; declared up to Write, the last field the loader calls
#[repr(C)]
pub struct SerialIoProtocol {
    revision: u32,
    reset: UnusedFunctionPointer,
    set_attributes: UnusedFunctionPointer,
    set_control: UnusedFunctionPointer,
    get_control: UnusedFunctionPointer,
    write: WriteFunction,
}

// SAFETY: this GUID identifies EFI_SERIAL_IO_PROTOCOL, the structure declared above
unsafe impl Protocol for SerialIoProtocol {
    // UEFI 2.11, 12.8.1
    const GUID: Guid = Guid::new(
        0xBB25_CF6F,
        0xF1D4,
        0x11D2,
        [0x9a, 0x0c, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0xfd],
    );
}

impl SerialIoProtocol {
    /// Sends the bytes and returns how many the device took
    pub fn write(&self, bytes: &[u8]) -> Result<usize, Status> {
        let mut size = bytes.len();
        // SAFETY: `self` lives in a firmware protocol instance that is valid
        // while boot services run; `size` is the length of `bytes`, and both
        // outlive the call
        unsafe { (self.write)(self, &mut size, bytes.as_ptr().cast()) }.into_result()?;
        Ok(size)
    }
}

const _: () = assert!(offset_of!(SerialIoProtocol, write) == 40);

#[cfg(test)]
mod tests {
    use super::{SerialIoProtocol, WriteFunction};
    use crate::Status;
    use core::ffi::c_void;
    use core::mem::MaybeUninit;
    use core::slice;

    unsafe extern "efiapi" fn accept_only_uranium(
        _this: *const SerialIoProtocol,
        size: *mut usize,
        buffer: *const c_void,
    ) -> Status {
        // SAFETY: the test passes a live size
        let length = unsafe { *size };
        // SAFETY: the test passes a buffer of `length` bytes
        let bytes = unsafe { slice::from_raw_parts(buffer.cast::<u8>(), length) };
        if bytes == b"uranium" {
            Status::SUCCESS
        } else {
            Status::DEVICE_ERROR
        }
    }

    unsafe extern "efiapi" fn take_half(
        _this: *const SerialIoProtocol,
        size: *mut usize,
        _buffer: *const c_void,
    ) -> Status {
        // SAFETY: the test passes a live size
        unsafe { *size /= 2 };
        Status::SUCCESS
    }

    fn protocol_with(write: WriteFunction) -> SerialIoProtocol {
        let mut protocol = MaybeUninit::<SerialIoProtocol>::zeroed();
        // SAFETY: the pointer comes from `MaybeUninit`, so the place exists
        let slot = unsafe { &raw mut (*protocol.as_mut_ptr()).write };
        // SAFETY: `slot` points inside `protocol`
        unsafe { slot.write(write) };
        // SAFETY: every other field is an integer or a nullable raw pointer,
        // for which all-zero bytes are valid; the one function pointer was
        // written above
        unsafe { protocol.assume_init() }
    }

    #[test]
    fn write_passes_the_bytes_and_their_length() {
        let protocol = protocol_with(accept_only_uranium);
        assert_eq!(protocol.write(b"uranium"), Ok(7));
        assert_eq!(protocol.write(b"uraniu"), Err(Status::DEVICE_ERROR));
    }

    #[test]
    fn write_returns_the_count_the_device_took() {
        let protocol = protocol_with(take_half);
        assert_eq!(protocol.write(b"uranium"), Ok(3));
    }
}
