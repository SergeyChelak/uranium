use address::VirtualAddress;
use byte_fields::{Decode, Decoder, layout, structure};

use crate::{ElfError, Header};

structure! {
    /// One program header table entry, with the ELF names spelled out:
    /// `segment_type` is `p_type`, `file_size` is `p_filesz`, and so on
    pub(crate) struct RawProgramHeader {
        pub(crate) segment_type: u32,
        pub(crate) flags: u32,
        pub(crate) offset: u64,
        pub(crate) virtual_address: VirtualAddress,
        pub(crate) physical_address: u64,       // not always physical address, so keep u64
        pub(crate) file_size: u64,
        pub(crate) memory_size: u64,
        pub(crate) alignment: u64,
    }
}

impl RawProgramHeader {
    pub(crate) const SIZE: usize = 56;
}

layout!(RawProgramHeader, RawProgramHeader::SIZE, {
    segment_type: 0,
    flags: 4,
    offset: 8,
    virtual_address: 16,
    physical_address: 24,
    file_size: 32,
    memory_size: 40,
    alignment: 48,
});

// ELF-64 1.5 Draft 2, Table 17
const EXECUTE: u32 = 0x1;
const WRITE: u32 = 0x2;
const READ: u32 = 0x4;

/// The `p_type` code: what a program header entry describes. Ranges are
/// left to operating systems and processors, so any code is a value and
/// the named ones are those the base document defines
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentKind(u32);

impl SegmentKind {
    // ELF-64 1.5 Draft 2, Table 16
    pub const NULL: Self = Self(0);
    pub const LOAD: Self = Self(1);
    pub const DYNAMIC: Self = Self(2);
    pub const INTERPRETER: Self = Self(3);
    pub const NOTE: Self = Self(4);
    pub const PROGRAM_HEADER_TABLE: Self = Self(6);

    pub const fn code(self) -> u32 {
        self.0
    }
}

/// One entry of the program header table: `file_size` bytes at `offset` in
/// the file describe `memory_size` bytes at `virtual_address`; for a
/// loadable segment the bytes beyond the file's are zero
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub kind: SegmentKind,
    pub virtual_address: VirtualAddress,
    pub offset: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub alignment: u64,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

impl Segment {
    fn from_raw(raw: &RawProgramHeader, file_length: usize) -> Result<Self, ElfError> {
        let end = raw
            .offset
            .checked_add(raw.file_size)
            .ok_or(ElfError::Truncated)?;
        if usize::try_from(end).map_or(true, |end| end > file_length) {
            return Err(ElfError::Truncated);
        }
        let kind = SegmentKind(raw.segment_type);
        if kind == SegmentKind::LOAD
            && (raw.memory_size < raw.file_size
                || raw.virtual_address.checked_add(raw.memory_size).is_none())
        {
            return Err(ElfError::SegmentSize);
        }
        Ok(Self {
            kind,
            virtual_address: raw.virtual_address,
            offset: raw.offset,
            file_size: raw.file_size,
            memory_size: raw.memory_size,
            alignment: raw.alignment,
            readable: raw.flags & READ != 0,
            writable: raw.flags & WRITE != 0,
            executable: raw.flags & EXECUTE != 0,
        })
    }

    /// The address just past the segment's memory
    pub fn end(&self) -> VirtualAddress {
        self.virtual_address + self.memory_size
    }

    pub fn contains(&self, address: VirtualAddress) -> bool {
        self.virtual_address <= address && address < self.end()
    }
}

impl Header {
    /// Every entry of the program header table of the file the header came
    /// from, in table order; an entry or a segment that reaches past
    /// `bytes` is an error in its place
    pub fn segments<'a>(
        &self,
        bytes: &'a [u8],
    ) -> impl Iterator<Item = Result<Segment, ElfError>> + 'a {
        let offset = self.program_header_offset;
        let entry_size = u64::from(self.program_header_entry_size);
        (0..u64::from(self.program_header_count)).map(move |index| {
            let start = offset
                .checked_add(index.saturating_mul(entry_size))
                .and_then(|start| usize::try_from(start).ok())
                .ok_or(ElfError::Truncated)?;
            let entry = bytes.get(start..).ok_or(ElfError::Truncated)?;
            let raw = RawProgramHeader::decode(&mut Decoder::new(entry))?;
            Segment::from_raw(&raw, bytes.len())
        })
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use address::VirtualAddress;

    use super::{RawProgramHeader, SegmentKind};

    /// An entry of `kind`: `file_size` bytes at `offset` in the file,
    /// `memory_size` bytes at `virtual_address`
    pub(crate) fn segment(
        kind: SegmentKind,
        virtual_address: u64,
        offset: u64,
        file_size: u64,
        memory_size: u64,
        flags: u32,
    ) -> RawProgramHeader {
        RawProgramHeader {
            segment_type: kind.code(),
            flags,
            offset,
            virtual_address: VirtualAddress::new(virtual_address),
            physical_address: 0,
            file_size,
            memory_size,
            alignment: 0x1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::testing::segment;
    use super::*;
    use crate::header::testing::{header, image};

    fn loadable(offset: u64, file_size: u64, memory_size: u64, flags: u32) -> RawProgramHeader {
        segment(
            SegmentKind::LOAD,
            0xffff_ffff_8000_0000 + offset,
            offset,
            file_size,
            memory_size,
            flags,
        )
    }

    fn segments(bytes: &[u8]) -> Vec<Result<Segment, ElfError>> {
        Header::parse(bytes).unwrap().segments(bytes).collect()
    }

    #[test]
    fn segments_report_every_kind() {
        let interpreter = segment(SegmentKind::INTERPRETER, 0, 176, 4, 4, READ);
        let text = loadable(180, 4, 4, READ | EXECUTE);
        let bytes = image(&header(2), &[interpreter, text], b"ld.\0\x90\x90\x90\x90");
        assert_eq!(
            segments(&bytes),
            [
                Ok(Segment {
                    kind: SegmentKind::INTERPRETER,
                    virtual_address: VirtualAddress::new(0),
                    offset: 176,
                    file_size: 4,
                    memory_size: 4,
                    alignment: 0x1000,
                    readable: true,
                    writable: false,
                    executable: false,
                }),
                Ok(Segment {
                    kind: SegmentKind::LOAD,
                    virtual_address: VirtualAddress::new(0xffff_ffff_8000_00b4),
                    offset: 180,
                    file_size: 4,
                    memory_size: 4,
                    alignment: 0x1000,
                    readable: true,
                    writable: false,
                    executable: true,
                }),
            ]
        );
    }

    #[test]
    fn an_unknown_kind_is_a_value() {
        let mut note = loadable(120, 0, 0, 0);
        note.segment_type = 0x6474_e551;
        let bytes = image(&header(1), &[note], &[]);
        assert_eq!(segments(&bytes)[0].unwrap().kind.code(), 0x6474_e551);
    }

    #[test]
    fn segment_flags_become_permissions() {
        let data = loadable(120, 4, 16, READ | WRITE);
        let bytes = image(&header(1), &[data], &[1, 2, 3, 4]);
        let segment = segments(&bytes).remove(0).unwrap();
        assert!(segment.readable && segment.writable && !segment.executable);
        assert_eq!((segment.file_size, segment.memory_size), (4, 16));
    }

    #[test]
    fn segments_are_read_by_entry_size() {
        let mut raw = header(2);
        raw.program_header_entry_size = 64;
        let first = loadable(192, 2, 2, READ);
        let second = loadable(194, 2, 2, READ);
        let bytes = image(&raw, &[first, second], &[1, 2, 3, 4]);
        let offsets: Vec<u64> = segments(&bytes)
            .into_iter()
            .map(|segment| segment.unwrap().offset)
            .collect();
        assert_eq!(offsets, [192, 194]);
    }

    #[test]
    fn truncated_program_headers_are_an_error() {
        let text = loadable(120, 8, 8, READ);
        let bytes = image(&header(1), &[text], &[0u8; 8]);
        let header = Header::parse(&bytes).unwrap();
        let cut: Vec<Result<Segment, ElfError>> = header.segments(&bytes[..100]).collect();
        assert_eq!(cut, [Err(ElfError::Truncated)]);
    }

    #[test]
    fn an_entry_past_the_file_is_truncated_whatever_its_kind() {
        let text = loadable(120, 16, 16, READ);
        let bytes = image(&header(1), &[text], &[0u8; 8]);
        assert_eq!(segments(&bytes), [Err(ElfError::Truncated)]);
        let mut note = loadable(120, 16, 16, READ);
        note.segment_type = SegmentKind::NOTE.code();
        let bytes = image(&header(1), &[note], &[0u8; 8]);
        assert_eq!(segments(&bytes), [Err(ElfError::Truncated)]);
    }

    #[test]
    fn memory_smaller_than_the_file_image_is_an_error() {
        let text = loadable(120, 8, 4, READ);
        let bytes = image(&header(1), &[text], &[0u8; 8]);
        assert_eq!(segments(&bytes), [Err(ElfError::SegmentSize)]);
    }

    #[test]
    fn memory_past_the_end_of_the_address_space_is_an_error() {
        let text = segment(SegmentKind::LOAD, u64::MAX - 7, 120, 8, 16, READ);
        let bytes = image(&header(1), &[text], &[0u8; 8]);
        assert_eq!(segments(&bytes), [Err(ElfError::SegmentSize)]);
    }

    #[test]
    fn a_segment_contains_its_memory_and_not_the_address_after_it() {
        let text = loadable(120, 8, 16, READ);
        let bytes = image(&header(1), &[text], &[0u8; 8]);
        let segment = segments(&bytes).remove(0).unwrap();
        assert_eq!(segment.end(), VirtualAddress::new(0xffff_ffff_8000_0088));
        assert!(segment.contains(VirtualAddress::new(0xffff_ffff_8000_0078)));
        assert!(segment.contains(VirtualAddress::new(0xffff_ffff_8000_0087)));
        assert!(!segment.contains(VirtualAddress::new(0xffff_ffff_8000_0088)));
        assert!(!segment.contains(VirtualAddress::new(0xffff_ffff_8000_0077)));
    }
}
