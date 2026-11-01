use std::vec::Vec;

use address::VirtualAddress;
use byte_fields::Encoder;

use crate::header::{RawHeader, RawIdentification};
use crate::program_header::{RawProgramHeader, SegmentType};

/// Where the kernel's linker script places the image
pub(crate) const IMAGE_BASE: u64 = 0xffff_ffff_8000_0000;

struct Entry {
    raw: RawProgramHeader,
    body: Vec<u8>,
}

/// File offsets are filled in by `build`, so a test never computes one
pub(crate) struct Image {
    header: RawHeader,
    entries: Vec<Entry>,
}

impl Image {
    pub(crate) fn new() -> Self {
        Self {
            header: RawHeader {
                identification: RawIdentification {
                    magic: *b"\x7fELF",
                    class: 2,
                    data: 1,
                    version: 1,
                    os_abi: 0,
                    abi_version: 0,
                    padding: [0; 7],
                },
                file_type: 2,
                machine: 62,
                version: 1,
                entry: VirtualAddress::new(IMAGE_BASE),
                program_header_offset: 64,
                section_header_offset: 0,
                flags: 0,
                header_size: 64,
                program_header_entry_size: 56,
                program_header_count: 0,
                section_header_entry_size: 64,
                section_header_count: 0,
                section_name_table_index: 0,
            },
            entries: Vec::new(),
        }
    }

    /// Changes the header after the entries added so far have counted
    /// themselves in
    pub(crate) fn header(mut self, change: impl FnOnce(&mut RawHeader)) -> Self {
        change(&mut self.header);
        self
    }

    pub(crate) fn segment(
        self,
        segment_type: SegmentType,
        virtual_address: u64,
        body: &[u8],
        memory_size: u64,
        flags: u32,
    ) -> Self {
        self.entry(
            RawProgramHeader {
                segment_type: segment_type.code(),
                flags,
                offset: 0,
                virtual_address: VirtualAddress::new(virtual_address),
                physical_address: 0,
                file_size: body.len() as u64,
                memory_size,
                alignment: 0x1000,
            },
            body.to_vec(),
        )
    }

    pub(crate) fn raw(self, raw: RawProgramHeader) -> Self {
        self.entry(raw, Vec::new())
    }

    fn entry(mut self, raw: RawProgramHeader, body: Vec<u8>) -> Self {
        self.entries.push(Entry { raw, body });
        self.header.program_header_count += 1;
        self
    }

    pub(crate) fn build(mut self) -> Vec<u8> {
        let entry_size = usize::from(self.header.program_header_entry_size);
        let mut offset = 64 + self.entries.len() * entry_size;
        for entry in &mut self.entries {
            if !entry.body.is_empty() {
                entry.raw.offset = offset as u64;
                offset += entry.body.len();
            }
        }
        let mut bytes = std::vec![0u8; offset];
        let mut encoder = Encoder::new(&mut bytes);
        encoder.write(&self.header).unwrap();
        for entry in &self.entries {
            let start = encoder.position();
            encoder.write(&entry.raw).unwrap();
            encoder
                .bytes(&std::vec![0u8; start + entry_size - encoder.position()])
                .unwrap();
        }
        for entry in &self.entries {
            encoder.bytes(&entry.body).unwrap();
        }
        bytes
    }
}
