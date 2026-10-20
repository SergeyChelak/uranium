use core::mem::offset_of;
use core::num::NonZeroU64;

use address::{AddressLayout, PhysicalAddress, VirtualAddress};

use crate::fields;
use crate::framebuffer::RawFramebuffer;
use crate::layout::layout;
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
        let u64_at = |field| fields::u64_at(bytes, field);
        let u32_at = |field| fields::u32_at(bytes, field);
        Some(Self {
            magic: u64_at(offset_of!(RawBootInformation, magic))?,
            version: u32_at(offset_of!(RawBootInformation, version))?,
            size: u32_at(offset_of!(RawBootInformation, size))?,
            physical_map_base: VirtualAddress::new(u64_at(offset_of!(
                RawBootInformation,
                physical_map_base
            ))?),
            kernel_image: RawRange::read_from(bytes, offset_of!(RawBootInformation, kernel_image))?,
            kernel_virtual_base: VirtualAddress::new(u64_at(offset_of!(
                RawBootInformation,
                kernel_virtual_base
            ))?),
            kernel_stack: RawRange::read_from(bytes, offset_of!(RawBootInformation, kernel_stack))?,
            page_tables: RawRange::read_from(bytes, offset_of!(RawBootInformation, page_tables))?,
            loader_data: RawRange::read_from(bytes, offset_of!(RawBootInformation, loader_data))?,
            memory_map: RawTable::read_from(bytes, offset_of!(RawBootInformation, memory_map))?,
            uefi_memory_map: RawTable::read_from(
                bytes,
                offset_of!(RawBootInformation, uefi_memory_map),
            )?,
            uefi_descriptor_version: u32_at(offset_of!(
                RawBootInformation,
                uefi_descriptor_version
            ))?,
            padding: u32_at(offset_of!(RawBootInformation, padding))?,
            uefi_system_table: NonZeroU64::new(u64_at(offset_of!(
                RawBootInformation,
                uefi_system_table
            ))?),
            rsdp: NonZeroU64::new(u64_at(offset_of!(RawBootInformation, rsdp))?),
            device_tree: RawRange::read_from(bytes, offset_of!(RawBootInformation, device_tree))?,
            framebuffer: RawFramebuffer::read_from(
                bytes,
                offset_of!(RawBootInformation, framebuffer),
            )?,
            command_line: RawRange::read_from(bytes, offset_of!(RawBootInformation, command_line))?,
            initial_ramdisk: RawRange::read_from(
                bytes,
                offset_of!(RawBootInformation, initial_ramdisk),
            )?,
            serial_console: RawSerialConsole::read_from(
                bytes,
                offset_of!(RawBootInformation, serial_console),
            )?,
        })
    }

    pub(crate) fn write_to(&self, block: &mut [u8]) {
        fields::put_u64(block, offset_of!(RawBootInformation, magic), self.magic);
        fields::put_u32(block, offset_of!(RawBootInformation, version), self.version);
        fields::put_u32(block, offset_of!(RawBootInformation, size), self.size);
        fields::put_u64(
            block,
            offset_of!(RawBootInformation, physical_map_base),
            self.physical_map_base.as_u64(),
        );
        self.kernel_image
            .write_to(block, offset_of!(RawBootInformation, kernel_image));
        fields::put_u64(
            block,
            offset_of!(RawBootInformation, kernel_virtual_base),
            self.kernel_virtual_base.as_u64(),
        );
        self.kernel_stack
            .write_to(block, offset_of!(RawBootInformation, kernel_stack));
        self.page_tables
            .write_to(block, offset_of!(RawBootInformation, page_tables));
        self.loader_data
            .write_to(block, offset_of!(RawBootInformation, loader_data));
        self.memory_map
            .write_to(block, offset_of!(RawBootInformation, memory_map));
        self.uefi_memory_map
            .write_to(block, offset_of!(RawBootInformation, uefi_memory_map));
        fields::put_u32(
            block,
            offset_of!(RawBootInformation, uefi_descriptor_version),
            self.uefi_descriptor_version,
        );
        fields::put_u32(block, offset_of!(RawBootInformation, padding), self.padding);
        fields::put_u64(
            block,
            offset_of!(RawBootInformation, uefi_system_table),
            self.uefi_system_table.map_or(0, NonZeroU64::get),
        );
        fields::put_u64(
            block,
            offset_of!(RawBootInformation, rsdp),
            self.rsdp.map_or(0, NonZeroU64::get),
        );
        self.device_tree
            .write_to(block, offset_of!(RawBootInformation, device_tree));
        self.framebuffer
            .write_to(block, offset_of!(RawBootInformation, framebuffer));
        self.command_line
            .write_to(block, offset_of!(RawBootInformation, command_line));
        self.initial_ramdisk
            .write_to(block, offset_of!(RawBootInformation, initial_ramdisk));
        self.serial_console
            .write_to(block, offset_of!(RawBootInformation, serial_console));
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
    use std::vec::Vec;

    use super::testing::TestLayout;
    use super::*;
    use crate::{MemoryKind, PixelFormat};

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
