use core::ffi::c_void;
use core::mem::offset_of;
use core::slice;

use crate::firmware_pointer::UnusedFirmwarePointer;
use crate::{Guid, Handle, Protocol};

// UEFI 2.11, 9.1.1; declared up to LoadOptions, the last field the loader reads
#[repr(C)]
pub struct LoadedImageProtocol {
    revision: u32,
    parent_handle: Option<Handle>,
    system_table: UnusedFirmwarePointer,
    device_handle: Option<Handle>,
    file_path: UnusedFirmwarePointer,
    reserved: UnusedFirmwarePointer,
    load_options_size: u32,
    load_options: *const c_void,
}

// SAFETY: this GUID identifies EFI_LOADED_IMAGE_PROTOCOL, the structure declared above
unsafe impl Protocol for LoadedImageProtocol {
    // UEFI 2.11, 9.1.1
    const GUID: Guid = Guid::new(
        0x5B1B_31A1,
        0x9562,
        0x11d2,
        [0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
    );
}

impl LoadedImageProtocol {
    /// The device the image was loaded from; `None` when the firmware did
    /// not load it from a device
    pub fn device_handle(&self) -> Option<Handle> {
        self.device_handle
    }

    /// The bytes the boot manager or shell attached to the image, in the
    /// encoding they chose; `None` when the image was started without any
    pub fn load_options(&self) -> Option<&[u8]> {
        if self.load_options.is_null() || self.load_options_size == 0 {
            return None;
        }
        let size = self.load_options_size as usize;
        // SAFETY: the firmware set `load_options` to a buffer of
        // `load_options_size` bytes that lives as long as this instance
        Some(unsafe { slice::from_raw_parts(self.load_options.cast::<u8>(), size) })
    }
}

const _: () = assert!(offset_of!(LoadedImageProtocol, device_handle) == 24);
const _: () = assert!(offset_of!(LoadedImageProtocol, load_options_size) == 48);
const _: () = assert!(offset_of!(LoadedImageProtocol, load_options) == 56);

#[cfg(test)]
mod tests {
    use super::LoadedImageProtocol;
    use crate::Handle;
    use core::mem::MaybeUninit;

    fn protocol_with(
        device_handle: Option<Handle>,
        load_options: Option<&[u8]>,
    ) -> LoadedImageProtocol {
        // SAFETY: every field is an integer or a nullable pointer, for which
        // all-zero bytes are valid
        let mut protocol = unsafe { MaybeUninit::<LoadedImageProtocol>::zeroed().assume_init() };
        protocol.device_handle = device_handle;
        if let Some(options) = load_options {
            protocol.load_options_size = u32::try_from(options.len()).unwrap();
            protocol.load_options = options.as_ptr().cast();
        }
        protocol
    }

    #[test]
    fn device_handle_is_none_when_null() {
        let protocol = protocol_with(None, None);
        assert_eq!(protocol.device_handle(), None);
    }

    #[test]
    fn device_handle_is_the_handle_the_firmware_wrote() {
        let handle = Handle::dangling();
        let protocol = protocol_with(Some(handle), None);
        assert_eq!(protocol.device_handle(), Some(handle));
    }

    #[test]
    fn load_options_slice_has_the_declared_length() {
        let options = b"k=v\0";
        let protocol = protocol_with(None, Some(options));
        assert_eq!(protocol.load_options(), Some(&options[..]));
    }

    #[test]
    fn load_options_are_none_when_the_image_was_started_without_any() {
        let protocol = protocol_with(None, None);
        assert_eq!(protocol.load_options(), None);
    }
}
