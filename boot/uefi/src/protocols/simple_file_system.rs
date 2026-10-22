use core::mem::offset_of;
use core::ptr;

use crate::protocols::file::{File, FileProtocol};
use crate::{Guid, Protocol, Status};

// UEFI 2.11, 13.4.2
type OpenVolumeFunction =
    unsafe extern "efiapi" fn(*const SimpleFileSystemProtocol, *mut *const FileProtocol) -> Status;

// UEFI 2.11, 13.4.1
#[repr(C)]
pub struct SimpleFileSystemProtocol {
    revision: u64,
    open_volume: OpenVolumeFunction,
}

// SAFETY: this GUID identifies EFI_SIMPLE_FILE_SYSTEM_PROTOCOL, the structure declared above
unsafe impl Protocol for SimpleFileSystemProtocol {
    // UEFI 2.11, 13.4.1
    const GUID: Guid = Guid::new(
        0x964e_5b22,
        0x6459,
        0x11d2,
        [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );
}

impl SimpleFileSystemProtocol {
    /// Opens the root directory of the volume
    pub fn open_volume(&self) -> Result<File<'_>, Status> {
        let mut root: *const FileProtocol = ptr::null();
        // SAFETY: `self` lives in a firmware protocol instance that is valid
        // while boot services run, and `root` outlives the call
        unsafe { (self.open_volume)(self, &mut root) }.into_result()?;
        // SAFETY: the firmware opened `root` for this caller, who now owns it
        unsafe { File::from_raw(root) }.ok_or(Status::DEVICE_ERROR)
    }
}

const _: () = assert!(offset_of!(SimpleFileSystemProtocol, open_volume) == 8);

#[cfg(test)]
mod tests {
    use super::SimpleFileSystemProtocol;
    use crate::Status;
    use crate::protocols::file::FileProtocol;
    use core::mem;
    use core::ptr::{self, NonNull};

    unsafe extern "efiapi" fn hand_back_a_root(
        _this: *const SimpleFileSystemProtocol,
        root: *mut *const FileProtocol,
    ) -> Status {
        // SAFETY: the test passes a live out-pointer
        unsafe { *root = NonNull::<FileProtocol>::dangling().as_ptr() };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn hand_back_nothing(
        _this: *const SimpleFileSystemProtocol,
        root: *mut *const FileProtocol,
    ) -> Status {
        // SAFETY: the test passes a live out-pointer
        unsafe { *root = ptr::null() };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn refuse(
        _this: *const SimpleFileSystemProtocol,
        root: *mut *const FileProtocol,
    ) -> Status {
        // SAFETY: the test passes a live out-pointer
        unsafe { *root = ptr::null() };
        Status::UNSUPPORTED
    }

    #[test]
    fn open_volume_returns_the_root_directory() {
        let file_system = SimpleFileSystemProtocol {
            revision: 0,
            open_volume: hand_back_a_root,
        };
        let root = file_system.open_volume().unwrap();
        // The stand-in root has no table behind it to close
        mem::forget(root);
    }

    #[test]
    fn open_volume_reports_the_firmware_error() {
        let file_system = SimpleFileSystemProtocol {
            revision: 0,
            open_volume: refuse,
        };
        assert_eq!(file_system.open_volume().err(), Some(Status::UNSUPPORTED));
    }

    #[test]
    fn open_volume_rejects_a_null_root() {
        let file_system = SimpleFileSystemProtocol {
            revision: 0,
            open_volume: hand_back_nothing,
        };
        assert_eq!(file_system.open_volume().err(), Some(Status::DEVICE_ERROR));
    }
}
