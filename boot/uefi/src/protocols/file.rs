use core::ffi::c_void;
use core::mem::{MaybeUninit, offset_of, size_of};
use core::ptr::{self, NonNull};

use crate::firmware_pointer::UnusedFunctionPointer;
use crate::protocols::file_information::{FILE_INFORMATION_ID, FileInformationBuffer};
use crate::ucs2::encode_terminated;
use crate::{Guid, Status};

// UEFI 2.11, 13.5.2
type OpenFunction = unsafe extern "efiapi" fn(
    *const FileProtocol,
    *mut *const FileProtocol,
    *const u16,
    u64,
    u64,
) -> Status;

// UEFI 2.11, 13.5.3
type CloseFunction = unsafe extern "efiapi" fn(*const FileProtocol) -> Status;

// UEFI 2.11, 13.5.5
type ReadFunction =
    unsafe extern "efiapi" fn(*const FileProtocol, *mut usize, *mut c_void) -> Status;

// UEFI 2.11, 13.5.13
type GetInfoFunction =
    unsafe extern "efiapi" fn(*const FileProtocol, *const Guid, *mut usize, *mut c_void) -> Status;

// UEFI 2.11, 13.5.2
const FILE_MODE_READ: u64 = 1;

/// Code units of a path `File::open` accepts; a longer one is rejected
const PATH_CAPACITY: usize = 255;

// UEFI 2.11, 13.5.1; declared up to GetInfo, the last field the loader calls
#[repr(C)]
pub(crate) struct FileProtocol {
    revision: u64,
    open: OpenFunction,
    close: CloseFunction,
    delete: UnusedFunctionPointer,
    read: ReadFunction,
    write: UnusedFunctionPointer,
    get_position: UnusedFunctionPointer,
    set_position: UnusedFunctionPointer,
    get_info: GetInfoFunction,
}

/// An open file or directory; closed on drop
pub struct File {
    protocol: NonNull<FileProtocol>,
}

impl File {
    /// Takes ownership of a file handle the firmware opened; `None` when it
    /// is null
    ///
    /// # Safety
    ///
    /// `protocol` must be what the firmware wrote for `OpenVolume` or `Open`,
    /// and nobody else may close it
    pub(crate) unsafe fn from_raw(protocol: *const FileProtocol) -> Option<File> {
        NonNull::new(protocol.cast_mut()).map(|protocol| File { protocol })
    }

    /// Opens `path` (UEFI form, `\` separators) for reading relative to this
    /// directory
    pub fn open(&self, path: &str) -> Result<File, Status> {
        let mut buffer = [0; PATH_CAPACITY + 1];
        let name = encode_terminated(path, &mut buffer).ok_or(Status::INVALID_PARAMETER)?;
        let mut opened: *const FileProtocol = ptr::null();
        // SAFETY: `name` is null-terminated, and `opened` outlives the call
        unsafe {
            (self.protocol().open)(self.as_raw(), &mut opened, name.as_ptr(), FILE_MODE_READ, 0)
        }
        .into_result()?;
        // SAFETY: the firmware opened `opened` for this caller, who now owns it
        unsafe { File::from_raw(opened) }.ok_or(Status::DEVICE_ERROR)
    }

    /// The file size from `EFI_FILE_INFO`
    pub fn size(&self) -> Result<u64, Status> {
        let mut buffer = MaybeUninit::<FileInformationBuffer>::zeroed();
        let mut size = size_of::<FileInformationBuffer>();
        // SAFETY: `size` is the length of `buffer`, and both outlive the call
        unsafe {
            (self.protocol().get_info)(
                self.as_raw(),
                &FILE_INFORMATION_ID,
                &mut size,
                buffer.as_mut_ptr().cast(),
            )
        }
        .into_result()?;
        // SAFETY: all-zero bytes are valid for every field, and the firmware
        // wrote the file information over them
        let filled = unsafe { buffer.assume_init_ref() };
        Ok(filled.information.file_size)
    }

    /// Reads at the current position; `Ok(0)` at the end
    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, Status> {
        let mut size = buffer.len();
        // SAFETY: `size` is the length of `buffer`, and both outlive the call
        unsafe { (self.protocol().read)(self.as_raw(), &mut size, buffer.as_mut_ptr().cast()) }
            .into_result()?;
        Ok(size)
    }

    fn as_raw(&self) -> *const FileProtocol {
        self.protocol.as_ptr()
    }

    fn protocol(&self) -> &FileProtocol {
        // SAFETY: the firmware keeps the instance alive until `Close`, which
        // only `drop` calls
        unsafe { self.protocol.as_ref() }
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // SAFETY: the handle is still open, and nothing uses it after this call
        unsafe { (self.protocol().close)(self.as_raw()) };
    }
}

const _: () = assert!(offset_of!(FileProtocol, open) == 8);
const _: () = assert!(offset_of!(FileProtocol, close) == 16);
const _: () = assert!(offset_of!(FileProtocol, read) == 32);
const _: () = assert!(offset_of!(FileProtocol, get_info) == 64);

#[cfg(test)]
mod tests {
    use super::{
        CloseFunction, File, FileProtocol, GetInfoFunction, OpenFunction, PATH_CAPACITY,
        ReadFunction,
    };
    use crate::firmware_pointer::UnusedFunctionPointer;
    use crate::protocols::file_information::{FILE_INFORMATION_ID, FileInformation};
    use crate::{Guid, Status};
    use core::ffi::c_void;
    use core::mem::size_of;
    use core::ptr;
    use std::vec::Vec;

    const CLOSED: u64 = 1;
    const FILE_SIZE: u64 = 4096;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn units_at(name: *const u16) -> Vec<u16> {
        let mut units = Vec::new();
        for index in 0.. {
            // SAFETY: the string is null-terminated and the loop stops at the terminator
            let unit = unsafe { *name.add(index) };
            if unit == 0 {
                break;
            }
            units.push(unit);
        }
        units
    }

    unsafe extern "efiapi" fn open_boot_configuration(
        this: *const FileProtocol,
        opened: *mut *const FileProtocol,
        name: *const u16,
        mode: u64,
        attributes: u64,
    ) -> Status {
        if units_at(name) != units("\\URANIUM\\BOOT.CFG") || mode != 1 || attributes != 0 {
            return Status::NOT_FOUND;
        }
        // SAFETY: the test passes a live out-pointer
        unsafe { *opened = this };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn reject_everything(
        _this: *const FileProtocol,
        _opened: *mut *const FileProtocol,
        _name: *const u16,
        _mode: u64,
        _attributes: u64,
    ) -> Status {
        Status::DEVICE_ERROR
    }

    unsafe extern "efiapi" fn close_quietly(_this: *const FileProtocol) -> Status {
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn mark_closed(this: *const FileProtocol) -> Status {
        // SAFETY: the test owns the table behind `this` and reads it only
        // after the file is dropped
        unsafe { (*this.cast_mut()).revision = CLOSED };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn give_uranium(
        _this: *const FileProtocol,
        size: *mut usize,
        buffer: *mut c_void,
    ) -> Status {
        // SAFETY: the test passes a live size
        let given = b"uranium".len().min(unsafe { *size });
        // SAFETY: the test passes a buffer of at least `given` bytes
        unsafe { ptr::copy_nonoverlapping(b"uranium".as_ptr(), buffer.cast::<u8>(), given) };
        // SAFETY: the test passes a live size
        unsafe { *size = given };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn give_nothing(
        _this: *const FileProtocol,
        size: *mut usize,
        _buffer: *mut c_void,
    ) -> Status {
        // SAFETY: the test passes a live size
        unsafe { *size = 0 };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn describe_a_file(
        _this: *const FileProtocol,
        kind: *const Guid,
        size: *mut usize,
        buffer: *mut c_void,
    ) -> Status {
        // SAFETY: the test passes a live GUID
        if unsafe { *kind } != FILE_INFORMATION_ID {
            return Status::UNSUPPORTED;
        }
        // SAFETY: the test passes a live size
        if unsafe { *size } < size_of::<FileInformation>() {
            return Status::BUFFER_TOO_SMALL;
        }
        let information = FileInformation {
            file_size: FILE_SIZE,
            ..FileInformation::default()
        };
        // SAFETY: `buffer` holds at least `size_of::<FileInformation>()` bytes, checked above
        unsafe {
            buffer
                .cast::<FileInformation>()
                .write_unaligned(information)
        };
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn demand_more_room(
        _this: *const FileProtocol,
        _kind: *const Guid,
        _size: *mut usize,
        _buffer: *mut c_void,
    ) -> Status {
        Status::BUFFER_TOO_SMALL
    }

    fn table_with(
        open: OpenFunction,
        close: CloseFunction,
        read: ReadFunction,
        get_info: GetInfoFunction,
    ) -> FileProtocol {
        FileProtocol {
            revision: 0,
            open,
            close,
            delete: UnusedFunctionPointer::null(),
            read,
            write: UnusedFunctionPointer::null(),
            get_position: UnusedFunctionPointer::null(),
            set_position: UnusedFunctionPointer::null(),
            get_info,
        }
    }

    fn file_over(table: &mut FileProtocol) -> File {
        // SAFETY: the test keeps `table` alive and leaves it alone while the file lives
        unsafe { File::from_raw(table) }.unwrap()
    }

    #[test]
    fn open_encodes_the_path_in_ucs2() {
        let mut table = table_with(
            open_boot_configuration,
            close_quietly,
            give_nothing,
            demand_more_room,
        );
        let root = file_over(&mut table);
        let file = root.open("\\URANIUM\\BOOT.CFG").unwrap();
        assert_eq!(file.protocol, root.protocol);
        assert_eq!(
            root.open("\\URANIUM\\OTHER.CFG").err(),
            Some(Status::NOT_FOUND)
        );
    }

    #[test]
    fn a_long_path_is_rejected() {
        let mut table = table_with(
            reject_everything,
            close_quietly,
            give_nothing,
            demand_more_room,
        );
        let root = file_over(&mut table);
        let longest = "x".repeat(PATH_CAPACITY);
        assert_eq!(root.open(&longest).err(), Some(Status::DEVICE_ERROR));
        let too_long = "x".repeat(PATH_CAPACITY + 1);
        assert_eq!(root.open(&too_long).err(), Some(Status::INVALID_PARAMETER));
    }

    #[test]
    fn read_returns_the_byte_count() {
        let mut table = table_with(
            reject_everything,
            close_quietly,
            give_uranium,
            demand_more_room,
        );
        let file = file_over(&mut table);
        let mut buffer = [0; 16];
        assert_eq!(file.read(&mut buffer), Ok(7));
        assert_eq!(&buffer[..7], b"uranium");
        let mut short = [0; 3];
        assert_eq!(file.read(&mut short), Ok(3));
        assert_eq!(&short, b"ura");
    }

    #[test]
    fn read_returns_zero_at_the_end() {
        let mut table = table_with(
            reject_everything,
            close_quietly,
            give_nothing,
            demand_more_room,
        );
        let file = file_over(&mut table);
        assert_eq!(file.read(&mut [0; 16]), Ok(0));
    }

    #[test]
    fn size_reads_file_info() {
        let mut table = table_with(
            reject_everything,
            close_quietly,
            give_nothing,
            describe_a_file,
        );
        let file = file_over(&mut table);
        assert_eq!(file.size(), Ok(FILE_SIZE));
    }

    #[test]
    fn size_fails_when_the_information_does_not_fit() {
        let mut table = table_with(
            reject_everything,
            close_quietly,
            give_nothing,
            demand_more_room,
        );
        let file = file_over(&mut table);
        assert_eq!(file.size(), Err(Status::BUFFER_TOO_SMALL));
    }

    #[test]
    fn drop_closes_the_file() {
        let mut table = table_with(
            reject_everything,
            mark_closed,
            give_nothing,
            demand_more_room,
        );
        let file = file_over(&mut table);
        drop(file);
        assert_eq!(table.revision, CLOSED);
    }
}
