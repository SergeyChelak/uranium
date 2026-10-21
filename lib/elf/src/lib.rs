//! Readers for the ELF64 executable format over a byte slice, without
//! touching memory: [`ElfFile`] reads and checks the file header, locates the
//! program header table and walks its entries; [`ExecutableFile`] adds the
//! checks every loader of an executable makes, with the rule for where a
//! segment may lie supplied by that loader. The crate reads and reports;
//! whether a file may be loaded, and where, is the caller's decision: the
//! loader places the kernel, the kernel places a user program.

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
pub use program_header::{Segment, SegmentKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    /// The bytes do not start with the ELF magic number
    NotElf,
    /// Not a 64-bit file
    Not64Bit,
    /// Not little-endian
    NotLittleEndian,
    /// The identification or the header names a version other than the
    /// current one
    UnknownVersion,
    /// The file ends before a header, a table or a segment does
    Truncated,
    /// A program header entry is shorter than the structure it must hold
    ProgramHeaderTooShort,
    /// A loadable segment claims less memory than it has bytes in the file
    MemorySmallerThanFile,
    /// A loadable segment's memory runs past the end of the address space
    MemoryPastAddressSpace,
}

impl From<byte_fields::OutOfBounds> for ElfError {
    fn from(_: byte_fields::OutOfBounds) -> Self {
        Self::Truncated
    }
}
