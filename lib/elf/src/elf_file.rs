use byte_fields::{Decode, Decoder, Fields};

use crate::ElfError;
use crate::header::{CURRENT_VERSION, Header, RawHeader};
use crate::program_header::{ProgramHeaderTable, Segment};

/// An ELF file over its bytes: the header read and checked, the program
/// header table located within the bytes
#[derive(Clone, Copy)]
pub struct ElfFile<'a> {
    bytes: &'a [u8],
    header: Header,
    program_headers: ProgramHeaderTable<'a>,
}

impl<'a> ElfFile<'a> {
    /// Reads the header at the start of `bytes` and locates the program
    /// header table within them; the only checks are that the file is one
    /// this crate can read and that it is consistent with itself
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ElfError> {
        let raw = RawHeader::decode(&mut Decoder::new(bytes))?;
        raw.identification.check()?;
        if raw.version != CURRENT_VERSION {
            return Err(ElfError::UnknownVersion);
        }
        let program_headers = ProgramHeaderTable::locate(
            Fields::new(bytes),
            raw.program_header_offset,
            raw.program_header_entry_size,
            raw.program_header_count,
        )?;
        Ok(Self {
            bytes,
            header: Header::from_raw(&raw),
            program_headers,
        })
    }

    pub fn header(&self) -> Header {
        self.header
    }

    /// Every entry of the program header table, in table order; an entry
    /// whose bytes reach past the file is an error in its place
    pub fn segments(&self) -> impl Iterator<Item = Result<Segment, ElfError>> + 'a {
        let file = Fields::new(self.bytes);
        self.program_headers
            .entries()
            .map(move |raw| Segment::from_raw(&raw?, file))
    }

    /// The bytes the file holds for `segment`; a loadable segment's memory
    /// beyond them is zero
    pub fn segment_bytes(&self, segment: &Segment) -> &'a [u8] {
        Fields::new(self.bytes)
            .window(segment.offset, segment.file_size)
            .expect("a segment of this file lies within it")
            .bytes()
    }
}

#[cfg(test)]
mod tests {
    use address::VirtualAddress;

    use super::*;
    use crate::program_header::{EXECUTE, READ};
    use crate::testing::{IMAGE_BASE, Image};
    use crate::{FileType, Machine, SegmentKind};

    #[test]
    fn parses_a_minimal_header() {
        let bytes = Image::new().build();
        assert_eq!(
            ElfFile::parse(&bytes).map(|file| file.header()),
            Ok(Header {
                file_type: FileType::EXECUTABLE,
                machine: Machine::X86_64,
                entry: VirtualAddress::new(IMAGE_BASE),
            })
        );
    }

    #[test]
    fn rejects_what_it_cannot_read() {
        let bytes = Image::new()
            .header(|header| header.identification.magic[1] = b'3')
            .build();
        assert_eq!(ElfFile::parse(&bytes).err(), Some(ElfError::NotElf));

        let bytes = Image::new()
            .header(|header| header.identification.class = 1)
            .build();
        assert_eq!(ElfFile::parse(&bytes).err(), Some(ElfError::Not64Bit));

        let bytes = Image::new()
            .header(|header| header.identification.data = 2)
            .build();
        assert_eq!(
            ElfFile::parse(&bytes).err(),
            Some(ElfError::NotLittleEndian)
        );
    }

    #[test]
    fn rejects_another_version_in_either_place() {
        let bytes = Image::new()
            .header(|header| header.identification.version = 2)
            .build();
        assert_eq!(ElfFile::parse(&bytes).err(), Some(ElfError::UnknownVersion));

        let bytes = Image::new().header(|header| header.version = 2).build();
        assert_eq!(ElfFile::parse(&bytes).err(), Some(ElfError::UnknownVersion));
    }

    #[test]
    fn reports_file_type_and_machine_as_values() {
        let bytes = Image::new()
            .header(|header| {
                header.file_type = 3;
                header.machine = 183;
            })
            .build();
        let header = ElfFile::parse(&bytes).unwrap().header();
        assert_eq!(header.file_type, FileType::SHARED_OBJECT);
        assert_eq!(header.machine, Machine::AARCH64);

        let bytes = Image::new()
            .header(|header| {
                header.file_type = 0xfe00;
                header.machine = 3;
            })
            .build();
        let header = ElfFile::parse(&bytes).unwrap().header();
        assert_eq!(header.file_type.code(), 0xfe00);
        assert_eq!(header.machine.code(), 3);
        assert_ne!(header.machine, Machine::X86_64);
    }

    #[test]
    fn rejects_a_short_program_header_entry() {
        let bytes = Image::new()
            .header(|header| header.program_header_entry_size = 55)
            .build();
        assert_eq!(
            ElfFile::parse(&bytes).err(),
            Some(ElfError::ProgramHeaderTooShort)
        );
    }

    #[test]
    fn a_short_file_is_truncated() {
        let bytes = Image::new().build();
        assert_eq!(
            ElfFile::parse(&bytes[..63]).err(),
            Some(ElfError::Truncated)
        );
    }

    #[test]
    fn a_program_header_table_past_the_file_is_truncated() {
        let bytes = Image::new()
            .segment(SegmentKind::LOAD, IMAGE_BASE, &[], 8, READ)
            .header(|header| header.program_header_count = 3)
            .build();
        assert_eq!(ElfFile::parse(&bytes).err(), Some(ElfError::Truncated));

        let bytes = Image::new()
            .segment(SegmentKind::LOAD, IMAGE_BASE, &[], 8, READ)
            .header(|header| header.program_header_offset = u64::MAX)
            .build();
        assert_eq!(ElfFile::parse(&bytes).err(), Some(ElfError::Truncated));
    }

    #[test]
    fn segment_bytes_are_the_file_bytes() {
        let bytes = Image::new()
            .segment(
                SegmentKind::LOAD,
                IMAGE_BASE,
                &[1, 2, 3],
                0x100,
                READ | EXECUTE,
            )
            .build();
        let file = ElfFile::parse(&bytes).unwrap();
        let segment = file.segments().next().unwrap().unwrap();
        assert_eq!(file.segment_bytes(&segment), [1, 2, 3]);
    }
}
