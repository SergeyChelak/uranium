use core::num::NonZeroU64;

use address::{AddressLayout, PhysicalAddress, VirtualAddress};
use byte_fields::{Decode, Decoder, Encode, Encoder, OutOfBounds, layout};

use crate::framebuffer::RawFramebuffer;
use crate::memory_map::{self, RawMemoryRegion};
use crate::ranges::{RawRange, RawTable};
use crate::serial_console::RawSerialConsole;
use crate::{Framebuffer, MemoryRegion, PhysicalRange, SerialConsole, Table, UefiMemoryMap};

/// What the loader writes into memory and the kernel finds through the
/// register of its first argument
#[repr(C)]
pub struct RawBootInformation {
    pub(crate) magic: u64,
    pub(crate) version: u32,
    pub(crate) size: u32,
    pub(crate) physical_map_base: VirtualAddress,
    pub(crate) kernel_image: RawRange,
    pub(crate) kernel_virtual_base: VirtualAddress,
    pub(crate) kernel_stack: RawRange,
    pub(crate) page_tables: RawRange,
    pub(crate) loader_data: RawRange,
    pub(crate) memory_map: RawTable,
    pub(crate) uefi_memory_map: RawTable,
    pub(crate) uefi_descriptor_version: u32,
    pub(crate) padding: u32,
    pub(crate) uefi_system_table: Option<NonZeroU64>,
    pub(crate) rsdp: Option<NonZeroU64>,
    pub(crate) device_tree: RawRange,
    pub(crate) framebuffer: RawFramebuffer,
    pub(crate) command_line: RawRange,
    pub(crate) initial_ramdisk: RawRange,
    pub(crate) serial_console: RawSerialConsole,
}

impl RawBootInformation {
    pub(crate) const MAGIC: u64 = u64::from_le_bytes(*b"URANIUM0");
    pub(crate) const VERSION: u32 = 0;
    pub(crate) const SIZE: u32 = 264;

    /// The structure the loader left at the start of `bytes`; `None` when
    /// there are fewer bytes than the structure takes
    pub fn read_from(bytes: &[u8]) -> Option<Self> {
        Self::decode(&mut Decoder::new(bytes)).ok()
    }
}

impl Decode for RawBootInformation {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, OutOfBounds> {
        Ok(Self {
            magic: decoder.read()?,
            version: decoder.read()?,
            size: decoder.read()?,
            physical_map_base: decoder.read()?,
            kernel_image: decoder.read()?,
            kernel_virtual_base: decoder.read()?,
            kernel_stack: decoder.read()?,
            page_tables: decoder.read()?,
            loader_data: decoder.read()?,
            memory_map: decoder.read()?,
            uefi_memory_map: decoder.read()?,
            uefi_descriptor_version: decoder.read()?,
            padding: decoder.read()?,
            uefi_system_table: decoder.read()?,
            rsdp: decoder.read()?,
            device_tree: decoder.read()?,
            framebuffer: decoder.read()?,
            command_line: decoder.read()?,
            initial_ramdisk: decoder.read()?,
            serial_console: decoder.read()?,
        })
    }
}

impl Encode for RawBootInformation {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), OutOfBounds> {
        encoder.write(&self.magic)?;
        encoder.write(&self.version)?;
        encoder.write(&self.size)?;
        encoder.write(&self.physical_map_base)?;
        encoder.write(&self.kernel_image)?;
        encoder.write(&self.kernel_virtual_base)?;
        encoder.write(&self.kernel_stack)?;
        encoder.write(&self.page_tables)?;
        encoder.write(&self.loader_data)?;
        encoder.write(&self.memory_map)?;
        encoder.write(&self.uefi_memory_map)?;
        encoder.write(&self.uefi_descriptor_version)?;
        encoder.write(&self.padding)?;
        encoder.write(&self.uefi_system_table)?;
        encoder.write(&self.rsdp)?;
        encoder.write(&self.device_tree)?;
        encoder.write(&self.framebuffer)?;
        encoder.write(&self.command_line)?;
        encoder.write(&self.initial_ramdisk)?;
        encoder.write(&self.serial_console)
    }
}

layout!(RawBootInformation, RawBootInformation::SIZE as usize, {
    magic: 0,
    version: 8,
    size: 12,
    physical_map_base: 16,
    kernel_image: 24,
    kernel_virtual_base: 40,
    kernel_stack: 48,
    page_tables: 64,
    loader_data: 80,
    memory_map: 96,
    uefi_memory_map: 120,
    uefi_descriptor_version: 144,
    padding: 148,
    uefi_system_table: 152,
    rsdp: 160,
    device_tree: 168,
    framebuffer: 184,
    command_line: 216,
    initial_ramdisk: 232,
    serial_console: 248,
});

/// Which check of `BootInformation::decode` failed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootInformationError {
    Magic,
    Version,
    Size,
    PhysicalMapBase,
    EmptyRange(&'static str),
    MemoryMapEntrySize,
}

/// The loader's hand-off, checked once by `decode` and read in place
pub struct BootInformation<'a> {
    raw: &'a RawBootInformation,
}

impl<'a> BootInformation<'a> {
    pub fn decode<L: AddressLayout>(
        raw: &'a RawBootInformation,
    ) -> Result<Self, BootInformationError> {
        if raw.magic != RawBootInformation::MAGIC {
            return Err(BootInformationError::Magic);
        }
        if raw.version != RawBootInformation::VERSION {
            return Err(BootInformationError::Version);
        }
        if raw.size < RawBootInformation::SIZE {
            return Err(BootInformationError::Size);
        }
        if raw.physical_map_base != L::PHYSICAL_MAP_BASE {
            return Err(BootInformationError::PhysicalMapBase);
        }
        let required = [
            ("kernel_image", &raw.kernel_image),
            ("kernel_stack", &raw.kernel_stack),
            ("page_tables", &raw.page_tables),
            ("loader_data", &raw.loader_data),
        ];
        if let Some((name, _)) = required.into_iter().find(|(_, range)| range.size == 0) {
            return Err(BootInformationError::EmptyRange(name));
        }
        let entry_size = raw.memory_map.entry_size;
        if entry_size < RawMemoryRegion::SIZE || usize::try_from(entry_size).is_err() {
            return Err(BootInformationError::MemoryMapEntrySize);
        }
        Ok(Self { raw })
    }

    pub fn physical_map_base(&self) -> VirtualAddress {
        self.raw.physical_map_base
    }

    pub fn kernel_image(&self) -> PhysicalRange {
        required(&self.raw.kernel_image)
    }

    pub fn kernel_virtual_base(&self) -> VirtualAddress {
        self.raw.kernel_virtual_base
    }

    pub fn kernel_stack(&self) -> PhysicalRange {
        required(&self.raw.kernel_stack)
    }

    pub fn page_tables(&self) -> PhysicalRange {
        required(&self.raw.page_tables)
    }

    pub fn loader_data(&self) -> PhysicalRange {
        required(&self.raw.loader_data)
    }

    /// Where the memory map lies; read its bytes and pass them to
    /// `memory_regions`
    pub fn memory_map(&self) -> Table {
        Table::from_raw(&self.raw.memory_map)
    }

    pub fn memory_regions(&self, table: &'a [u8]) -> impl Iterator<Item = MemoryRegion> + 'a {
        let entry_size = usize::try_from(self.raw.memory_map.entry_size)
            .expect("the entry size was checked by decode");
        memory_map::regions(table, self.raw.memory_map.count, entry_size)
    }

    pub fn uefi_memory_map(&self) -> UefiMemoryMap {
        UefiMemoryMap {
            table: Table::from_raw(&self.raw.uefi_memory_map),
            descriptor_version: self.raw.uefi_descriptor_version,
        }
    }

    pub fn uefi_system_table(&self) -> Option<PhysicalAddress> {
        self.raw
            .uefi_system_table
            .map(|address| PhysicalAddress::new(address.get()))
    }

    pub fn rsdp(&self) -> Option<PhysicalAddress> {
        self.raw
            .rsdp
            .map(|address| PhysicalAddress::new(address.get()))
    }

    pub fn device_tree(&self) -> Option<PhysicalRange> {
        PhysicalRange::from_raw(&self.raw.device_tree)
    }

    pub fn framebuffer(&self) -> Option<Framebuffer> {
        Framebuffer::from_raw(&self.raw.framebuffer)
    }

    pub fn command_line(&self) -> Option<PhysicalRange> {
        PhysicalRange::from_raw(&self.raw.command_line)
    }

    pub fn initial_ramdisk(&self) -> Option<PhysicalRange> {
        PhysicalRange::from_raw(&self.raw.initial_ramdisk)
    }

    pub fn serial_console(&self) -> Option<SerialConsole> {
        SerialConsole::from_raw(&self.raw.serial_console)
    }
}

fn required(range: &RawRange) -> PhysicalRange {
    PhysicalRange::from_raw(range).expect("the required ranges were checked by decode")
}

#[cfg(test)]
pub(crate) mod testing {
    use address::{AddressLayout, VirtualAddress};

    pub(crate) struct TestLayout;

    impl AddressLayout for TestLayout {
        const VIRTUAL_ADDRESS_BIT_COUNT: u32 = 48;
        const PHYSICAL_ADDRESS_BIT_COUNT: u32 = 52;
        const KERNEL_HALF_START: VirtualAddress = VirtualAddress::new(0xffff_8000_0000_0000);
        const PHYSICAL_MAP_BASE: VirtualAddress = VirtualAddress::new(0xffff_8000_0000_0000);
    }
}

#[cfg(test)]
mod tests {
    use core::mem::offset_of;
    use std::vec::Vec;

    use byte_fields::Fields;

    use super::testing::TestLayout;
    use super::*;
    use crate::{MemoryKind, PixelFormat};

    fn filled(byte: u8) -> u64 {
        u64::from_le_bytes([byte; 8])
    }

    fn filled32(byte: u8) -> u32 {
        u32::from_le_bytes([byte; 4])
    }

    /// Every field carries its own byte pattern, so a field written at the
    /// wrong offset cannot pass for its neighbour
    fn distinct() -> RawBootInformation {
        RawBootInformation {
            magic: filled(0x01),
            version: filled32(0x02),
            size: filled32(0x03),
            physical_map_base: VirtualAddress::new(filled(0x04)),
            kernel_image: range(filled(0x05), filled(0x06)),
            kernel_virtual_base: VirtualAddress::new(filled(0x07)),
            kernel_stack: range(filled(0x08), filled(0x09)),
            page_tables: range(filled(0x0a), filled(0x0b)),
            loader_data: range(filled(0x0c), filled(0x0d)),
            memory_map: table(filled(0x0e), filled(0x0f), filled(0x10)),
            uefi_memory_map: table(filled(0x11), filled(0x12), filled(0x13)),
            uefi_descriptor_version: filled32(0x14),
            padding: filled32(0x15),
            uefi_system_table: NonZeroU64::new(filled(0x16)),
            rsdp: NonZeroU64::new(filled(0x17)),
            device_tree: range(filled(0x18), filled(0x19)),
            framebuffer: RawFramebuffer {
                base: PhysicalAddress::new(filled(0x1a)),
                size: filled(0x1b),
                width: filled32(0x1c),
                height: filled32(0x1d),
                stride: filled32(0x1e),
                pixel_format: filled32(0x1f),
            },
            command_line: range(filled(0x20), filled(0x21)),
            initial_ramdisk: range(filled(0x22), filled(0x23)),
            serial_console: RawSerialConsole {
                base: filled(0x24),
                kind: filled32(0x25),
                padding: filled32(0x26),
            },
        }
    }

    #[test]
    fn every_field_lands_at_its_documented_offset() {
        let raw = distinct();
        let mut block = [0u8; RawBootInformation::SIZE as usize];
        raw.encode(&mut Encoder::new(&mut block)).unwrap();
        let fields = Fields::new(&block);
        let u64_at = |offset: usize| fields.u64(offset).unwrap();
        let u32_at = |offset: usize| fields.u32(offset).unwrap();
        let range_at = |offset: usize, range: &RawRange| {
            assert_eq!(
                u64_at(offset + offset_of!(RawRange, base)),
                range.base.as_u64()
            );
            assert_eq!(u64_at(offset + offset_of!(RawRange, size)), range.size);
        };
        let table_at = |offset: usize, table: &RawTable| {
            assert_eq!(
                u64_at(offset + offset_of!(RawTable, base)),
                table.base.as_u64()
            );
            assert_eq!(u64_at(offset + offset_of!(RawTable, count)), table.count);
            assert_eq!(
                u64_at(offset + offset_of!(RawTable, entry_size)),
                table.entry_size
            );
        };

        assert_eq!(u64_at(offset_of!(RawBootInformation, magic)), raw.magic);
        assert_eq!(u32_at(offset_of!(RawBootInformation, version)), raw.version);
        assert_eq!(u32_at(offset_of!(RawBootInformation, size)), raw.size);
        assert_eq!(
            u64_at(offset_of!(RawBootInformation, physical_map_base)),
            raw.physical_map_base.as_u64()
        );
        range_at(
            offset_of!(RawBootInformation, kernel_image),
            &raw.kernel_image,
        );
        assert_eq!(
            u64_at(offset_of!(RawBootInformation, kernel_virtual_base)),
            raw.kernel_virtual_base.as_u64()
        );
        range_at(
            offset_of!(RawBootInformation, kernel_stack),
            &raw.kernel_stack,
        );
        range_at(
            offset_of!(RawBootInformation, page_tables),
            &raw.page_tables,
        );
        range_at(
            offset_of!(RawBootInformation, loader_data),
            &raw.loader_data,
        );
        table_at(offset_of!(RawBootInformation, memory_map), &raw.memory_map);
        table_at(
            offset_of!(RawBootInformation, uefi_memory_map),
            &raw.uefi_memory_map,
        );
        assert_eq!(
            u32_at(offset_of!(RawBootInformation, uefi_descriptor_version)),
            raw.uefi_descriptor_version
        );
        assert_eq!(u32_at(offset_of!(RawBootInformation, padding)), raw.padding);
        assert_eq!(
            u64_at(offset_of!(RawBootInformation, uefi_system_table)),
            raw.uefi_system_table.unwrap().get()
        );
        assert_eq!(
            u64_at(offset_of!(RawBootInformation, rsdp)),
            raw.rsdp.unwrap().get()
        );
        range_at(
            offset_of!(RawBootInformation, device_tree),
            &raw.device_tree,
        );
        let framebuffer = offset_of!(RawBootInformation, framebuffer);
        assert_eq!(
            u64_at(framebuffer + offset_of!(RawFramebuffer, base)),
            raw.framebuffer.base.as_u64()
        );
        assert_eq!(
            u64_at(framebuffer + offset_of!(RawFramebuffer, size)),
            raw.framebuffer.size
        );
        assert_eq!(
            u32_at(framebuffer + offset_of!(RawFramebuffer, width)),
            raw.framebuffer.width
        );
        assert_eq!(
            u32_at(framebuffer + offset_of!(RawFramebuffer, height)),
            raw.framebuffer.height
        );
        assert_eq!(
            u32_at(framebuffer + offset_of!(RawFramebuffer, stride)),
            raw.framebuffer.stride
        );
        assert_eq!(
            u32_at(framebuffer + offset_of!(RawFramebuffer, pixel_format)),
            raw.framebuffer.pixel_format
        );
        range_at(
            offset_of!(RawBootInformation, command_line),
            &raw.command_line,
        );
        range_at(
            offset_of!(RawBootInformation, initial_ramdisk),
            &raw.initial_ramdisk,
        );
        let serial = offset_of!(RawBootInformation, serial_console);
        assert_eq!(
            u64_at(serial + offset_of!(RawSerialConsole, base)),
            raw.serial_console.base
        );
        assert_eq!(
            u32_at(serial + offset_of!(RawSerialConsole, kind)),
            raw.serial_console.kind
        );
        assert_eq!(
            u32_at(serial + offset_of!(RawSerialConsole, padding)),
            raw.serial_console.padding
        );
    }

    fn range(base: u64, size: u64) -> RawRange {
        RawRange {
            base: PhysicalAddress::new(base),
            size,
        }
    }

    fn table(base: u64, count: u64, entry_size: u64) -> RawTable {
        RawTable {
            base: PhysicalAddress::new(base),
            count,
            entry_size,
        }
    }

    fn valid() -> RawBootInformation {
        RawBootInformation {
            magic: RawBootInformation::MAGIC,
            version: RawBootInformation::VERSION,
            size: RawBootInformation::SIZE,
            physical_map_base: TestLayout::PHYSICAL_MAP_BASE,
            kernel_image: range(0x10_0000, 0x20_0000),
            kernel_virtual_base: VirtualAddress::new(0xffff_ffff_8000_0000),
            kernel_stack: range(0x30_0000, 0x1_0000),
            page_tables: range(0x40_0000, 0x1_0000),
            loader_data: range(0x50_0000, 0x1000),
            memory_map: table(0x50_0200, 3, 24),
            uefi_memory_map: table(0x50_1000, 10, 48),
            uefi_descriptor_version: 1,
            padding: 0,
            uefi_system_table: NonZeroU64::new(0x7f00_0000),
            rsdp: NonZeroU64::new(0x7e00_0000),
            device_tree: range(0, 0),
            framebuffer: RawFramebuffer {
                base: PhysicalAddress::new(0),
                size: 0,
                width: 0,
                height: 0,
                stride: 0,
                pixel_format: 0,
            },
            command_line: range(0x50_2000, 16),
            initial_ramdisk: range(0, 0),
            serial_console: RawSerialConsole {
                base: 0,
                kind: 0,
                padding: 0,
            },
        }
    }

    fn decode(raw: &RawBootInformation) -> Result<BootInformation<'_>, BootInformationError> {
        BootInformation::decode::<TestLayout>(raw)
    }

    fn physical(base: u64, size: u64) -> PhysicalRange {
        PhysicalRange::new(PhysicalAddress::new(base), size).unwrap()
    }

    fn entry(base: u64, size: u64, kind: u32, entry_size: usize) -> Vec<u8> {
        let mut bytes = Vec::from(base.to_le_bytes());
        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(&kind.to_le_bytes());
        bytes.resize(entry_size, 0xee);
        bytes
    }

    #[test]
    fn decode_accepts_a_valid_block() {
        assert!(decode(&valid()).is_ok());
    }

    #[test]
    fn decode_rejects_a_bad_magic() {
        let mut raw = valid();
        raw.magic = u64::from_le_bytes(*b"URANIUM1");
        assert_eq!(decode(&raw).err(), Some(BootInformationError::Magic));
    }

    #[test]
    fn decode_rejects_another_version() {
        let mut raw = valid();
        raw.version = 1;
        assert_eq!(decode(&raw).err(), Some(BootInformationError::Version));
    }

    #[test]
    fn decode_rejects_a_short_size_and_accepts_a_longer_one() {
        let mut raw = valid();
        raw.size = RawBootInformation::SIZE - 1;
        assert_eq!(decode(&raw).err(), Some(BootInformationError::Size));
        raw.size = RawBootInformation::SIZE + 64;
        assert!(decode(&raw).is_ok());
    }

    #[test]
    fn decode_rejects_a_foreign_physical_map_base() {
        let mut raw = valid();
        raw.physical_map_base = VirtualAddress::new(0xffff_0000_0000_0000);
        assert_eq!(
            decode(&raw).err(),
            Some(BootInformationError::PhysicalMapBase)
        );
    }

    #[test]
    fn decode_rejects_an_empty_required_range() {
        let mut raw = valid();
        raw.kernel_stack = range(0x30_0000, 0);
        assert_eq!(
            decode(&raw).err(),
            Some(BootInformationError::EmptyRange("kernel_stack"))
        );
    }

    #[test]
    fn decode_rejects_a_short_memory_map_entry() {
        let mut raw = valid();
        raw.memory_map = table(0x50_0200, 3, 16);
        assert_eq!(
            decode(&raw).err(),
            Some(BootInformationError::MemoryMapEntrySize)
        );
    }

    #[test]
    fn required_fields_decode_to_values() {
        let raw = valid();
        let information = decode(&raw).unwrap();
        assert_eq!(
            information.physical_map_base(),
            TestLayout::PHYSICAL_MAP_BASE
        );
        assert_eq!(information.kernel_image(), physical(0x10_0000, 0x20_0000));
        assert_eq!(
            information.kernel_virtual_base(),
            VirtualAddress::new(0xffff_ffff_8000_0000)
        );
        assert_eq!(information.kernel_stack(), physical(0x30_0000, 0x1_0000));
        assert_eq!(information.page_tables(), physical(0x40_0000, 0x1_0000));
        assert_eq!(information.loader_data(), physical(0x50_0000, 0x1000));
        assert_eq!(
            information.memory_map(),
            Table {
                base: PhysicalAddress::new(0x50_0200),
                count: 3,
                entry_size: 24,
            }
        );
        assert_eq!(information.memory_map().size(), 72);
        assert_eq!(
            information.uefi_memory_map(),
            UefiMemoryMap {
                table: Table {
                    base: PhysicalAddress::new(0x50_1000),
                    count: 10,
                    entry_size: 48,
                },
                descriptor_version: 1,
            }
        );
        assert_eq!(
            information.uefi_system_table(),
            Some(PhysicalAddress::new(0x7f00_0000))
        );
        assert_eq!(information.rsdp(), Some(PhysicalAddress::new(0x7e00_0000)));
        assert_eq!(information.command_line(), Some(physical(0x50_2000, 16)));
    }

    #[test]
    fn absent_fields_decode_to_none() {
        let mut raw = valid();
        raw.uefi_system_table = None;
        raw.rsdp = None;
        raw.command_line = range(0x50_2000, 0);
        let information = decode(&raw).unwrap();
        assert_eq!(information.uefi_system_table(), None);
        assert_eq!(information.rsdp(), None);
        assert_eq!(information.device_tree(), None);
        assert_eq!(information.framebuffer(), None);
        assert_eq!(information.command_line(), None);
        assert_eq!(information.initial_ramdisk(), None);
        assert_eq!(information.serial_console(), None);
    }

    #[test]
    fn present_optional_fields_decode_to_values() {
        let mut raw = valid();
        raw.device_tree = range(0x6000_0000, 0x1_0000);
        raw.initial_ramdisk = range(0x7000_0000, 0x10_0000);
        raw.framebuffer = RawFramebuffer {
            base: PhysicalAddress::new(0x8000_0000),
            size: 0x30_0000,
            width: 1024,
            height: 768,
            stride: 1024,
            pixel_format: 2,
        };
        let information = decode(&raw).unwrap();
        assert_eq!(
            information.device_tree(),
            Some(physical(0x6000_0000, 0x1_0000))
        );
        assert_eq!(
            information.initial_ramdisk(),
            Some(physical(0x7000_0000, 0x10_0000))
        );
        assert_eq!(
            information.framebuffer(),
            Some(Framebuffer {
                range: physical(0x8000_0000, 0x30_0000),
                width: 1024,
                height: 768,
                stride: 1024,
                pixel_format: PixelFormat::Bgr8,
            })
        );
    }

    #[test]
    fn serial_console_decodes_both_kinds() {
        let mut raw = valid();
        raw.serial_console = RawSerialConsole {
            base: 0x3f8,
            kind: 1,
            padding: 0,
        };
        assert_eq!(
            decode(&raw).unwrap().serial_console(),
            Some(SerialConsole::Uart16550 { port: 0x3f8 })
        );
        raw.serial_console = RawSerialConsole {
            base: 0x0900_0000,
            kind: 2,
            padding: 0,
        };
        assert_eq!(
            decode(&raw).unwrap().serial_console(),
            Some(SerialConsole::Pl011 {
                base: PhysicalAddress::new(0x0900_0000)
            })
        );
    }

    #[test]
    fn serial_console_is_absent_for_an_unknown_kind_or_an_impossible_port() {
        let mut raw = valid();
        raw.serial_console = RawSerialConsole {
            base: 0x3f8,
            kind: 7,
            padding: 0,
        };
        assert_eq!(decode(&raw).unwrap().serial_console(), None);
        raw.serial_console = RawSerialConsole {
            base: 0x1_0000,
            kind: 1,
            padding: 0,
        };
        assert_eq!(decode(&raw).unwrap().serial_console(), None);
    }

    #[test]
    fn memory_regions_step_by_entry_size() {
        let mut raw = valid();
        raw.memory_map = table(0x50_0200, 2, 32);
        let mut bytes = entry(0x1000, 0x9000, 1, 32);
        bytes.extend(entry(0xa000, 0x6000, 7, 32));
        let information = decode(&raw).unwrap();
        let regions: Vec<MemoryRegion> = information.memory_regions(&bytes).collect();
        assert_eq!(
            regions,
            [
                MemoryRegion {
                    range: physical(0x1000, 0x9000),
                    kind: MemoryKind::Usable,
                },
                MemoryRegion {
                    range: physical(0xa000, 0x6000),
                    kind: MemoryKind::KernelImage,
                },
            ]
        );
    }

    #[test]
    fn memory_regions_stop_at_the_count() {
        let mut raw = valid();
        raw.memory_map = table(0x50_0200, 1, 24);
        let mut bytes = entry(0x1000, 0x9000, 1, 24);
        bytes.extend(entry(0xa000, 0x6000, 1, 24));
        let information = decode(&raw).unwrap();
        assert_eq!(information.memory_regions(&bytes).count(), 1);
    }

    #[test]
    fn memory_regions_skip_an_entry_without_memory() {
        let mut raw = valid();
        raw.memory_map = table(0x50_0200, 2, 24);
        let mut bytes = entry(0x1000, 0, 1, 24);
        bytes.extend(entry(0xa000, 0x6000, 13, 24));
        let information = decode(&raw).unwrap();
        let regions: Vec<MemoryRegion> = information.memory_regions(&bytes).collect();
        assert_eq!(
            regions,
            [MemoryRegion {
                range: physical(0xa000, 0x6000),
                kind: MemoryKind::Reserved,
            }]
        );
    }
}
