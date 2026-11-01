use address::VirtualAddress;
use byte_fields::{Fields, OutOfBounds, layout, structure};

use crate::ElfError;

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

// ELF-64 1.5 Draft 2, Table 17
pub(crate) const EXECUTE: u32 = 0x1;
pub(crate) const WRITE: u32 = 0x2;
pub(crate) const READ: u32 = 0x4;

/// The program header table as located in the file: `count` entries of
/// `entry_size` bytes each, the first [`RawProgramHeader::SIZE`] of which
/// are the structure
#[derive(Clone, Copy)]
pub(crate) struct ProgramHeaderTable<'a> {
    bytes: Fields<'a>,
    entry_size: usize,
    count: usize,
}

impl<'a> ProgramHeaderTable<'a> {
    pub(crate) fn locate(
        file: Fields<'a>,
        offset: u64,
        entry_size: u16,
        count: u16,
    ) -> Result<Self, ElfError> {
        if usize::from(entry_size) < RawProgramHeader::SIZE {
            return Err(ElfError::ProgramHeaderTooShort);
        }
        let bytes = file.window(offset, u64::from(entry_size) * u64::from(count))?;
        Ok(Self {
            bytes,
            entry_size: usize::from(entry_size),
            count: usize::from(count),
        })
    }

    pub(crate) fn entries(
        self,
    ) -> impl Iterator<Item = Result<RawProgramHeader, OutOfBounds>> + 'a {
        let mut cursor = self.bytes.at(0);
        let padding = self.entry_size - RawProgramHeader::SIZE;
        (0..self.count).map(move |_| {
            let raw = cursor.read::<RawProgramHeader>()?;
            cursor.skip(padding)?;
            Ok(raw)
        })
    }
}

/// The `p_type` code; any code is a value, because the specification leaves
/// ranges to operating systems and processors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentType(u32);

impl SegmentType {
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

/// For a loadable segment the memory beyond the bytes of the file is zero
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment<'a> {
    segment_type: SegmentType,
    virtual_address: VirtualAddress,
    end: VirtualAddress,
    bytes: &'a [u8],
    memory_size: u64,
    alignment: u64,
    readable: bool,
    writable: bool,
    executable: bool,
}

impl<'a> Segment<'a> {
    pub(crate) fn from_raw(raw: &RawProgramHeader, file: Fields<'a>) -> Result<Self, ElfError> {
        let bytes = file.window(raw.offset, raw.file_size)?.bytes();
        let end = raw
            .virtual_address
            .checked_add(raw.memory_size)
            .ok_or(ElfError::MemoryPastAddressSpace)?;
        let segment_type = SegmentType(raw.segment_type);
        if segment_type == SegmentType::LOAD && raw.memory_size < raw.file_size {
            return Err(ElfError::MemorySmallerThanFile);
        }
        Ok(Self {
            segment_type,
            virtual_address: raw.virtual_address,
            end,
            bytes,
            memory_size: raw.memory_size,
            alignment: raw.alignment,
            readable: raw.flags & READ != 0,
            writable: raw.flags & WRITE != 0,
            executable: raw.flags & EXECUTE != 0,
        })
    }

    pub fn segment_type(&self) -> SegmentType {
        self.segment_type
    }

    pub fn virtual_address(&self) -> VirtualAddress {
        self.virtual_address
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub fn memory_size(&self) -> u64 {
        self.memory_size
    }

    pub fn alignment(&self) -> u64 {
        self.alignment
    }

    pub fn is_readable(&self) -> bool {
        self.readable
    }

    pub fn is_writable(&self) -> bool {
        self.writable
    }

    pub fn is_executable(&self) -> bool {
        self.executable
    }

    pub fn end(&self) -> VirtualAddress {
        self.end
    }

    pub fn contains(&self, address: VirtualAddress) -> bool {
        self.virtual_address <= address && address < self.end
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::*;
    use crate::ElfFile;
    use crate::testing::{IMAGE_BASE, Image};

    fn segments(bytes: &[u8]) -> Vec<Result<Segment<'_>, ElfError>> {
        ElfFile::parse(bytes).unwrap().segments().collect()
    }

    #[test]
    fn segments_report_every_kind() {
        let bytes = Image::new()
            .segment(SegmentType::INTERPRETER, 0, b"ld.\0", 4, READ)
            .segment(SegmentType::LOAD, IMAGE_BASE, &[0x90; 4], 4, READ | EXECUTE)
            .build();
        assert_eq!(
            segments(&bytes),
            [
                Ok(Segment {
                    segment_type: SegmentType::INTERPRETER,
                    virtual_address: VirtualAddress::new(0),
                    end: VirtualAddress::new(4),
                    bytes: b"ld.\0",
                    memory_size: 4,
                    alignment: 0x1000,
                    readable: true,
                    writable: false,
                    executable: false,
                }),
                Ok(Segment {
                    segment_type: SegmentType::LOAD,
                    virtual_address: VirtualAddress::new(IMAGE_BASE),
                    end: VirtualAddress::new(IMAGE_BASE + 4),
                    bytes: &[0x90; 4],
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
        let stack = Image::new().raw(RawProgramHeader {
            segment_type: 0x6474_e551,
            flags: READ | WRITE,
            offset: 0,
            virtual_address: VirtualAddress::new(0),
            physical_address: 0,
            file_size: 0,
            memory_size: 0,
            alignment: 16,
        });
        let bytes = stack.build();
        let segment = segments(&bytes).remove(0).unwrap();
        assert_eq!(segment.segment_type().code(), 0x6474_e551);
        assert_eq!(segment.alignment(), 16);
    }

    #[test]
    fn segment_flags_become_permissions() {
        let bytes = Image::new()
            .segment(
                SegmentType::LOAD,
                IMAGE_BASE,
                &[1, 2, 3, 4],
                16,
                READ | WRITE,
            )
            .build();
        let segment = segments(&bytes).remove(0).unwrap();
        assert!(segment.is_readable() && segment.is_writable() && !segment.is_executable());
        assert_eq!(segment.bytes(), [1, 2, 3, 4]);
        assert_eq!(segment.memory_size(), 16);
    }

    #[test]
    fn segments_are_read_by_entry_size() {
        let bytes = Image::new()
            .header(|header| header.program_header_entry_size = 64)
            .segment(SegmentType::LOAD, IMAGE_BASE, &[1, 2], 2, READ)
            .segment(SegmentType::LOAD, IMAGE_BASE + 0x1000, &[3, 4], 2, READ)
            .build();
        let bodies: Vec<&[u8]> = segments(&bytes)
            .into_iter()
            .map(|segment| segment.unwrap().bytes())
            .collect();
        assert_eq!(bodies, [[1, 2], [3, 4]]);
    }

    #[test]
    fn a_file_cut_inside_a_segment_is_truncated() {
        let mut bytes = Image::new()
            .segment(SegmentType::NOTE, 0, &[0u8; 16], 16, READ)
            .build();
        bytes.truncate(bytes.len() - 8);
        assert_eq!(segments(&bytes), [Err(ElfError::Truncated)]);
    }

    #[test]
    fn memory_smaller_than_the_file_image_is_an_error() {
        let bytes = Image::new()
            .segment(SegmentType::LOAD, IMAGE_BASE, &[0u8; 8], 4, READ)
            .build();
        assert_eq!(segments(&bytes), [Err(ElfError::MemorySmallerThanFile)]);
    }

    #[test]
    fn memory_past_the_end_of_the_address_space_is_an_error() {
        let bytes = Image::new()
            .segment(SegmentType::LOAD, u64::MAX - 7, &[0u8; 8], 16, READ)
            .build();
        assert_eq!(segments(&bytes), [Err(ElfError::MemoryPastAddressSpace)]);
    }

    #[test]
    fn memory_past_the_end_of_the_address_space_is_an_error_for_every_kind() {
        let bytes = Image::new()
            .segment(SegmentType::NOTE, u64::MAX - 7, &[], 16, READ)
            .build();
        assert_eq!(segments(&bytes), [Err(ElfError::MemoryPastAddressSpace)]);
    }

    #[test]
    fn a_segment_contains_its_memory_and_not_the_address_after_it() {
        let bytes = Image::new()
            .segment(SegmentType::LOAD, IMAGE_BASE, &[0u8; 8], 16, READ)
            .build();
        let segment = segments(&bytes).remove(0).unwrap();
        assert_eq!(segment.end(), VirtualAddress::new(IMAGE_BASE + 16));
        assert!(segment.contains(VirtualAddress::new(IMAGE_BASE)));
        assert!(segment.contains(VirtualAddress::new(IMAGE_BASE + 15)));
        assert!(!segment.contains(VirtualAddress::new(IMAGE_BASE + 16)));
        assert!(!segment.contains(VirtualAddress::new(IMAGE_BASE - 1)));
    }
}
