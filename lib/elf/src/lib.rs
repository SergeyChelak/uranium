//! Readers for the ELF64 executable format: the file header and the program
//! header table it points at, over a byte slice and without touching memory.
//! The crate reads and reports; whether a file may be loaded, and where, is
//! the caller's decision: the loader places the kernel, the kernel places a
//! user program.

#![no_std]

#[cfg(test)]
extern crate std;

mod header;
mod program_header;

pub use header::{FileType, Header, Machine};
pub use program_header::{Segment, SegmentKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    /// The bytes do not start with the ELF magic number
    Magic,
    /// Not a 64-bit file
    Class,
    /// Not little-endian
    Endianness,
    /// Neither the identification nor the header carries the current version
    Version,
    /// The file ends before a header, a table or a segment does
    Truncated,
    /// A program header entry is shorter than the structure it must hold
    ProgramHeaderSize,
    /// A loadable segment claims less memory than it has bytes in the file,
    /// or memory that runs past the end of the address space
    SegmentSize,
}

impl From<byte_fields::OutOfBounds> for ElfError {
    fn from(_: byte_fields::OutOfBounds) -> Self {
        Self::Truncated
    }
}
