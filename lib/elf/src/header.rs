use address::VirtualAddress;
use byte_fields::{Decode, Decoder, layout, structure};

use crate::ElfError;
use crate::program_header::RawProgramHeader;

structure! {
    /// The file header, with the ELF names spelled out: `identification` is
    /// `e_ident`, `program_header_offset` is `e_phoff`, and so on
    pub(crate) struct RawHeader {
        pub(crate) identification: [u8; 16],
        pub(crate) file_type: u16,
        pub(crate) machine: u16,
        pub(crate) version: u32,
        pub(crate) entry: VirtualAddress,
        pub(crate) program_header_offset: u64,
        pub(crate) section_header_offset: u64,
        pub(crate) flags: u32,
        pub(crate) header_size: u16,
        pub(crate) program_header_entry_size: u16,
        pub(crate) program_header_count: u16,
        pub(crate) section_header_entry_size: u16,
        pub(crate) section_header_count: u16,
        pub(crate) section_name_table_index: u16,
    }
}

layout!(RawHeader, 64, {
    identification: 0,
    file_type: 16,
    machine: 18,
    version: 20,
    entry: 24,
    program_header_offset: 32,
    section_header_offset: 40,
    flags: 48,
    header_size: 52,
    program_header_entry_size: 54,
    program_header_count: 56,
    section_header_entry_size: 58,
    section_header_count: 60,
    section_name_table_index: 62,
});

// ELF-64 1.5 Draft 2, section 3: e_ident[0..4]
const MAGIC: [u8; 4] = *b"\x7fELF";
// ELF-64 1.5 Draft 2, Table 2: indices into e_ident
const CLASS: usize = 4;
const DATA: usize = 5;
const IDENTIFICATION_VERSION: usize = 6;
// ELF-64 1.5 Draft 2, Table 3
const CLASS_64: u8 = 2;
// ELF-64 1.5 Draft 2, Table 4
const DATA_LITTLE_ENDIAN: u8 = 1;
// ELF-64 1.5 Draft 2, section 3: EV_CURRENT
const CURRENT_VERSION: u32 = 1;
// ELF-64 1.5 Draft 2, Table 6
const EXECUTABLE: u16 = 2;

/// The processors an executable can be built for; the discriminant is the
/// `e_machine` code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Machine {
    // x86-64 psABI 0.99.7, 4.1.1
    X86_64 = 62,
    // AAELF64 2025Q4, 5.2
    Aarch64 = 183,
}

impl Machine {
    fn from_code(code: u16) -> Result<Self, ElfError> {
        match code {
            62 => Ok(Self::X86_64),
            183 => Ok(Self::Aarch64),
            _ => Err(ElfError::Machine),
        }
    }

    pub fn code(self) -> u16 {
        self as u16
    }
}

/// What the loader needs from the file header, checked on the way in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub machine: Machine,
    pub entry: VirtualAddress,
    pub(crate) program_header_offset: u64,
    pub(crate) program_header_entry_size: u16,
    pub(crate) program_header_count: u16,
}

impl Header {
    /// Reads and checks the header at the start of `bytes`: a 64-bit
    /// little-endian executable of the current version for a known machine,
    /// whose program header table lies within `bytes`
    pub fn parse(bytes: &[u8]) -> Result<Self, ElfError> {
        let raw = RawHeader::decode(&mut Decoder::new(bytes))?;
        let identification = raw.identification;
        if identification[..MAGIC.len()] != MAGIC {
            return Err(ElfError::Magic);
        }
        if identification[CLASS] != CLASS_64 {
            return Err(ElfError::Class);
        }
        if identification[DATA] != DATA_LITTLE_ENDIAN {
            return Err(ElfError::Endianness);
        }
        if u32::from(identification[IDENTIFICATION_VERSION]) != CURRENT_VERSION
            || raw.version != CURRENT_VERSION
        {
            return Err(ElfError::Version);
        }
        if raw.file_type != EXECUTABLE {
            return Err(ElfError::Type);
        }
        let machine = Machine::from_code(raw.machine)?;
        if usize::from(raw.program_header_entry_size) < RawProgramHeader::SIZE {
            return Err(ElfError::ProgramHeaderSize);
        }
        let header = Self {
            machine,
            entry: raw.entry,
            program_header_offset: raw.program_header_offset,
            program_header_entry_size: raw.program_header_entry_size,
            program_header_count: raw.program_header_count,
        };
        if header.program_header_table_end() > bytes.len() {
            return Err(ElfError::Truncated);
        }
        Ok(header)
    }

    /// Where the program header table ends, saturating so that an absurd
    /// offset reads as "past any file"
    fn program_header_table_end(&self) -> usize {
        let size = usize::from(self.program_header_entry_size)
            .saturating_mul(usize::from(self.program_header_count));
        usize::try_from(self.program_header_offset)
            .map_or(usize::MAX, |offset| offset.saturating_add(size))
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use std::vec::Vec;

    use address::VirtualAddress;
    use byte_fields::Encoder;

    use super::RawHeader;
    use crate::program_header::RawProgramHeader;

    pub(crate) const X86_64: u16 = 62;

    /// A header for `program_headers` that start right behind the header
    pub(crate) fn header(program_headers: usize) -> RawHeader {
        RawHeader {
            identification: *b"\x7fELF\x02\x01\x01\0\0\0\0\0\0\0\0\0",
            file_type: 2,
            machine: X86_64,
            version: 1,
            entry: VirtualAddress::new(0xffff_ffff_8000_0000),
            program_header_offset: 64,
            section_header_offset: 0,
            flags: 0,
            header_size: 64,
            program_header_entry_size: 56,
            program_header_count: u16::try_from(program_headers).unwrap(),
            section_header_entry_size: 64,
            section_header_count: 0,
            section_name_table_index: 0,
        }
    }

    /// The header, then the program headers, then `body` as the segment bytes
    pub(crate) fn image(
        header: &RawHeader,
        program_headers: &[RawProgramHeader],
        body: &[u8],
    ) -> Vec<u8> {
        let entry_size = usize::from(header.program_header_entry_size);
        let mut bytes = std::vec![0u8; 64 + program_headers.len() * entry_size + body.len()];
        let mut encoder = Encoder::new(&mut bytes);
        encoder.write(header).unwrap();
        for program_header in program_headers {
            let start = encoder.position();
            encoder.write(program_header).unwrap();
            encoder
                .bytes(&std::vec![0u8; start + entry_size - encoder.position()])
                .unwrap();
        }
        encoder.bytes(body).unwrap();
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{header, image};
    use super::*;

    #[test]
    fn parses_a_minimal_header() {
        let bytes = image(&header(0), &[], &[]);
        assert_eq!(
            Header::parse(&bytes),
            Ok(Header {
                machine: Machine::X86_64,
                entry: VirtualAddress::new(0xffff_ffff_8000_0000),
                program_header_offset: 64,
                program_header_entry_size: 56,
                program_header_count: 0,
            })
        );
    }

    #[test]
    fn rejects_a_bad_magic() {
        let mut bytes = image(&header(0), &[], &[]);
        bytes[1] = b'3';
        assert_eq!(Header::parse(&bytes), Err(ElfError::Magic));
    }

    #[test]
    fn rejects_32_bit() {
        let mut bytes = image(&header(0), &[], &[]);
        bytes[4] = 1;
        assert_eq!(Header::parse(&bytes), Err(ElfError::Class));
    }

    #[test]
    fn rejects_big_endian() {
        let mut bytes = image(&header(0), &[], &[]);
        bytes[5] = 2;
        assert_eq!(Header::parse(&bytes), Err(ElfError::Endianness));
    }

    #[test]
    fn rejects_another_version() {
        let mut bytes = image(&header(0), &[], &[]);
        bytes[6] = 2;
        assert_eq!(Header::parse(&bytes), Err(ElfError::Version));
        let mut raw = header(0);
        raw.version = 2;
        assert_eq!(
            Header::parse(&image(&raw, &[], &[])),
            Err(ElfError::Version)
        );
    }

    #[test]
    fn rejects_shared_objects() {
        let mut raw = header(0);
        raw.file_type = 3;
        assert_eq!(Header::parse(&image(&raw, &[], &[])), Err(ElfError::Type));
    }

    #[test]
    fn knows_both_machines_and_no_other() {
        let mut raw = header(0);
        raw.machine = 183;
        assert_eq!(
            Header::parse(&image(&raw, &[], &[])).map(|header| header.machine),
            Ok(Machine::Aarch64)
        );
        raw.machine = 3;
        assert_eq!(
            Header::parse(&image(&raw, &[], &[])),
            Err(ElfError::Machine)
        );
        assert_eq!(Machine::X86_64.code(), 62);
        assert_eq!(Machine::Aarch64.code(), 183);
    }

    #[test]
    fn rejects_a_short_program_header_entry() {
        let mut raw = header(0);
        raw.program_header_entry_size = 55;
        assert_eq!(
            Header::parse(&image(&raw, &[], &[])),
            Err(ElfError::ProgramHeaderSize)
        );
    }

    #[test]
    fn a_short_file_is_truncated() {
        let bytes = image(&header(0), &[], &[]);
        assert_eq!(Header::parse(&bytes[..63]), Err(ElfError::Truncated));
    }

    #[test]
    fn a_program_header_table_past_the_file_is_truncated() {
        let mut raw = header(2);
        raw.program_header_count = 3;
        let bytes = image(&raw, &[], &[0u8; 2 * 56]);
        assert_eq!(Header::parse(&bytes), Err(ElfError::Truncated));
        raw.program_header_count = 2;
        raw.program_header_offset = u64::MAX;
        assert_eq!(
            Header::parse(&image(&raw, &[], &[])),
            Err(ElfError::Truncated)
        );
    }
}
