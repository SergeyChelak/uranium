use core::mem::{align_of, offset_of, size_of};

use crate::Guid;

// UEFI 2.11, 13.5.16
pub(crate) const FILE_INFORMATION_ID: Guid = Guid::new(
    0x0957_6e92,
    0x6d3f,
    0x11d2,
    [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
);

/// Code units of a file name `File::size` makes room for; a longer name
/// fails with `BUFFER_TOO_SMALL`
const NAME_CAPACITY: usize = 255;

// UEFI 2.11, 8.3.1; the loader reads no field of it, so only its size and
// alignment are declared
#[cfg_attr(test, derive(Default))]
#[repr(C, align(4))]
pub(crate) struct Time([u8; 16]);

// UEFI 2.11, 13.5.16; the fixed part, which the null-terminated name follows
#[cfg_attr(test, derive(Default))]
#[repr(C)]
pub(crate) struct FileInformation {
    pub(crate) size: u64,
    pub(crate) file_size: u64,
    pub(crate) physical_size: u64,
    pub(crate) create_time: Time,
    pub(crate) last_access_time: Time,
    pub(crate) modification_time: Time,
    pub(crate) attribute: u64,
}

/// What `GetInfo` fills: the fixed part and a name of up to `NAME_CAPACITY`
/// code units with its terminator
#[repr(C)]
pub(crate) struct FileInformationBuffer {
    pub(crate) information: FileInformation,
    name: [u16; NAME_CAPACITY + 1],
}

const _: () = assert!(size_of::<Time>() == 16);
const _: () = assert!(align_of::<Time>() == 4);
const _: () = assert!(offset_of!(FileInformation, file_size) == 8);
const _: () = assert!(offset_of!(FileInformation, attribute) == 72);
const _: () = assert!(size_of::<FileInformation>() == 80);
const _: () = assert!(offset_of!(FileInformationBuffer, name) == 80);
