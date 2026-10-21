//! Readers for the ELF64 executable format: the file header and the
//! loadable segments it points at, over a byte slice and without touching
//! memory. What to do with a segment is the caller's decision: the loader
//! copies it into physical memory, the kernel maps it into an address
//! space.

#![no_std]

#[cfg(test)]
extern crate std;

mod header;
mod program_header;

pub use header::{Header, Machine};
pub use program_header::Segment;

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
    /// Not an executable file
    Type,
    /// A machine this crate does not know
    Machine,
    /// The file ends before a header, a table or a segment does
    Truncated,
    /// A program header entry is shorter than the structure it must hold
    ProgramHeaderSize,
    /// A segment claims less memory than it has bytes in the file
    SegmentSize,
}

impl From<byte_fields::OutOfBounds> for ElfError {
    fn from(_: byte_fields::OutOfBounds) -> Self {
        Self::Truncated
    }
}
