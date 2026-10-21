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
        pub(crate) physical_address: u64,
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

// ELF-64 1.5 Draft 2, Table 16
const LOADABLE: u32 = 1;
// ELF-64 1.5 Draft 2, Table 17
const EXECUTE: u32 = 0x1;
const WRITE: u32 = 0x2;
const READ: u32 = 0x4;

/// A loadable segment: `file_size` bytes at `offset` in the file become the
/// first bytes of `memory_size` at `virtual_address`, the rest are zero
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub virtual_address: VirtualAddress,
    pub offset: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

impl Segment {
    /// `None` for an entry that is not a loadable segment
    fn from_raw(raw: &RawProgramHeader, file_length: usize) -> Result<Option<Self>, ElfError> {
        if raw.segment_type != LOADABLE {
            return Ok(None);
        }
        if raw.memory_size < raw.file_size {
            return Err(ElfError::SegmentSize);
        }
        let end = raw
            .offset
            .checked_add(raw.file_size)
            .ok_or(ElfError::Truncated)?;
        if usize::try_from(end).map_or(true, |end| end > file_length) {
            return Err(ElfError::Truncated);
        }
        Ok(Some(Self {
            virtual_address: raw.virtual_address,
            offset: raw.offset,
            file_size: raw.file_size,
            memory_size: raw.memory_size,
            readable: raw.flags & READ != 0,
            writable: raw.flags & WRITE != 0,
            executable: raw.flags & EXECUTE != 0,
        }))
    }
}

impl Header {
    /// The loadable segments of the file the header came from, in table
    /// order; other entry types are skipped, and an entry or a segment that
    /// reaches past `bytes` is an error in its place
    pub fn load_segments<'a>(
        &self,
        bytes: &'a [u8],
    ) -> impl Iterator<Item = Result<Segment, ElfError>> + 'a {
        let offset = self.program_header_offset;
        let entry_size = u64::from(self.program_header_entry_size);
        (0..u64::from(self.program_header_count))
            .map(move |index| {
                let start = offset
                    .checked_add(index.saturating_mul(entry_size))
                    .and_then(|start| usize::try_from(start).ok())
                    .ok_or(ElfError::Truncated)?;
                let entry = bytes.get(start..).ok_or(ElfError::Truncated)?;
                let raw = RawProgramHeader::decode(&mut Decoder::new(entry))?;
                Segment::from_raw(&raw, bytes.len())
            })
            .filter_map(Result::transpose)
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::*;
    use crate::header::testing::{header, image};

    fn loadable(offset: u64, file_size: u64, memory_size: u64, flags: u32) -> RawProgramHeader {
        RawProgramHeader {
            segment_type: LOADABLE,
            flags,
            offset,
            virtual_address: VirtualAddress::new(0xffff_ffff_8000_0000 + offset),
            physical_address: 0,
            file_size,
            memory_size,
            alignment: 0x1000,
        }
    }

    fn segments(bytes: &[u8]) -> Vec<Result<Segment, ElfError>> {
        Header::parse(bytes).unwrap().load_segments(bytes).collect()
    }

    #[test]
    fn load_segments_skips_other_types() {
        let mut note = loadable(0, 0, 0, 0);
        note.segment_type = 4;
        let text = loadable(176, 8, 8, READ | EXECUTE);
        let bytes = image(&header(2), &[note, text], &[7u8; 8]);
        assert_eq!(
            segments(&bytes),
            [Ok(Segment {
                virtual_address: VirtualAddress::new(0xffff_ffff_8000_00b0),
                offset: 176,
                file_size: 8,
                memory_size: 8,
                readable: true,
                writable: false,
                executable: true,
            })]
        );
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
        let cut: Vec<Result<Segment, ElfError>> = header.load_segments(&bytes[..100]).collect();
        assert_eq!(cut, [Err(ElfError::Truncated)]);
    }

    #[test]
    fn a_segment_past_the_file_is_truncated() {
        let text = loadable(120, 16, 16, READ);
        let bytes = image(&header(1), &[text], &[0u8; 8]);
        assert_eq!(segments(&bytes), [Err(ElfError::Truncated)]);
    }

    #[test]
    fn memory_smaller_than_the_file_image_is_an_error() {
        let text = loadable(120, 8, 4, READ);
        let bytes = image(&header(1), &[text], &[0u8; 8]);
        assert_eq!(segments(&bytes), [Err(ElfError::SegmentSize)]);
    }
}
