use address::VirtualAddress;
use byte_fields::{layout, structure};

use crate::ElfError;

structure! {
    /// The identification bytes that open the file, `e_ident`, with the
    /// `EI_*` indices of the specification turned into fields
    pub(crate) struct RawIdentification {
        pub(crate) magic: [u8; 4],
        pub(crate) class: u8,
        pub(crate) data: u8,
        pub(crate) version: u8,
        pub(crate) os_abi: u8,
        pub(crate) abi_version: u8,
        pub(crate) padding: [u8; 7],
    }
}

// ELF-64 1.5 Draft 2, Table 2
layout!(RawIdentification, 16, align 1, {
    magic: 0,
    class: 4,
    data: 5,
    version: 6,
    os_abi: 7,
    abi_version: 8,
    padding: 9,
});

structure! {
    /// The file header, with the ELF names spelled out: `identification` is
    /// `e_ident`, `program_header_offset` is `e_phoff`, and so on
    pub(crate) struct RawHeader {
        pub(crate) identification: RawIdentification,
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

// ELF-64 1.5 Draft 2, section 3: e_ident[EI_MAG0] through e_ident[EI_MAG3]
const MAGIC: [u8; 4] = *b"\x7fELF";
// ELF-64 1.5 Draft 2, Table 3
const CLASS_64: u8 = 2;
// ELF-64 1.5 Draft 2, Table 4
const DATA_LITTLE_ENDIAN: u8 = 1;
// ELF-64 1.5 Draft 2, section 3: EV_CURRENT
pub(crate) const CURRENT_VERSION: u32 = 1;

impl RawIdentification {
    /// Whether this crate can read the file: ELF, 64-bit, little-endian, of
    /// the current version
    pub(crate) fn check(&self) -> Result<(), ElfError> {
        if self.magic != MAGIC {
            return Err(ElfError::NotElf);
        }
        if self.class != CLASS_64 {
            return Err(ElfError::Not64Bit);
        }
        if self.data != DATA_LITTLE_ENDIAN {
            return Err(ElfError::NotLittleEndian);
        }
        if u32::from(self.version) != CURRENT_VERSION {
            return Err(ElfError::UnknownVersion);
        }
        Ok(())
    }
}

/// The `e_type` code: what kind of object the file is. The specification
/// leaves ranges to operating systems and processors, so any code is a
/// value here and the named ones are those the base document defines
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileType(u16);

impl FileType {
    // ELF-64 1.5 Draft 2, Table 6
    pub const NONE: Self = Self(0);
    pub const RELOCATABLE: Self = Self(1);
    pub const EXECUTABLE: Self = Self(2);
    pub const SHARED_OBJECT: Self = Self(3);
    pub const CORE: Self = Self(4);

    pub const fn code(self) -> u16 {
        self.0
    }
}

/// The `e_machine` code: the processor the file is built for. Only the two
/// Uranium runs on are named; any other code is still a value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Machine(u16);

impl Machine {
    // x86-64 psABI 0.99.7, 4.1.1
    pub const X86_64: Self = Self(62);
    // AAELF64 2025Q4, 5.2
    pub const AARCH64: Self = Self(183);

    pub const fn code(self) -> u16 {
        self.0
    }
}

/// What the file header says about the file as a whole
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub file_type: FileType,
    pub machine: Machine,
    pub entry: VirtualAddress,
}

impl Header {
    pub(crate) fn from_raw(raw: &RawHeader) -> Self {
        Self {
            file_type: FileType(raw.file_type),
            machine: Machine(raw.machine),
            entry: raw.entry,
        }
    }
}
