#![no_std]

#[cfg(test)]
extern crate std;

mod elf_file;
mod executable_file;
mod header;
mod program_header;
#[cfg(test)]
pub(crate) mod testing;

pub use elf_file::ElfFile;
pub use executable_file::{ExecutableError, ExecutableFile};
pub use header::{FileType, Header, Machine};
pub use program_header::{Segment, SegmentType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    NotElf,
    Not64Bit,
    NotLittleEndian,
    UnknownVersion,
    Truncated,
    ProgramHeaderTooShort,
    MemorySmallerThanFile,
    MemoryPastAddressSpace,
}

impl From<byte_fields::OutOfBounds> for ElfError {
    fn from(_: byte_fields::OutOfBounds) -> Self {
        Self::Truncated
    }
}
