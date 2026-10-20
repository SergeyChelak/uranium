use core::num::NonZeroU64;

use address::{PhysicalAddress, VirtualAddress};

use crate::fields;
use crate::framebuffer::RawFramebuffer;
use crate::information::RawBootInformation;
use crate::memory_map::RawMemoryRegion;
use crate::ranges::{RawRange, RawTable};
use crate::serial_console::RawSerialConsole;
use crate::{MemoryRegion, PhysicalRange, SerialConsole, UefiMemoryMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildError {
    MissingField(&'static str),
    /// The structure must start on an eight-byte boundary
    BlockUnaligned,
    BlockTooSmall {
        needed: usize,
    },
}

/// Collects the typed values the loader has and lays the wire format out in
/// the loader's data block; `write_into` refuses to write before every
/// required value is set
pub struct BootInformationBuilder<'a> {
    physical_map_base: VirtualAddress,
    kernel_image: Option<(PhysicalRange, VirtualAddress)>,
    kernel_stack: Option<PhysicalRange>,
    page_tables: Option<PhysicalRange>,
    loader_data: Option<PhysicalRange>,
    memory_regions: Option<&'a [MemoryRegion]>,
    uefi_memory_map: Option<UefiMemoryMap>,
    uefi_system_table: Option<PhysicalAddress>,
    rsdp: Option<PhysicalAddress>,
    serial_console: Option<SerialConsole>,
    command_line: &'a str,
}

impl<'a> BootInformationBuilder<'a> {
    pub fn new(physical_map_base: VirtualAddress) -> Self {
        Self {
            physical_map_base,
            kernel_image: None,
            kernel_stack: None,
            page_tables: None,
            loader_data: None,
            memory_regions: None,
            uefi_memory_map: None,
            uefi_system_table: None,
            rsdp: None,
            serial_console: None,
            command_line: "",
        }
    }

    pub fn kernel_image(self, range: PhysicalRange, virtual_base: VirtualAddress) -> Self {
        Self {
            kernel_image: Some((range, virtual_base)),
            ..self
        }
    }

    pub fn kernel_stack(self, range: PhysicalRange) -> Self {
        Self {
            kernel_stack: Some(range),
            ..self
        }
    }

    pub fn page_tables(self, range: PhysicalRange) -> Self {
        Self {
            page_tables: Some(range),
            ..self
        }
    }

    /// The whole block the loader hands to the kernel: the structure, the
    /// tables laid out behind it and the firmware's memory map
    pub fn loader_data(self, range: PhysicalRange) -> Self {
        Self {
            loader_data: Some(range),
            ..self
        }
    }

    pub fn memory_regions(self, regions: &'a [MemoryRegion]) -> Self {
        Self {
            memory_regions: Some(regions),
            ..self
        }
    }

    /// Where the firmware's own map already lies; it is not copied
    pub fn uefi_memory_map(self, map: UefiMemoryMap) -> Self {
        Self {
            uefi_memory_map: Some(map),
            ..self
        }
    }

    /// Address zero means no table, the same as `None`
    pub fn uefi_system_table(self, address: Option<PhysicalAddress>) -> Self {
        Self {
            uefi_system_table: address,
            ..self
        }
    }

    /// Address zero means no table, the same as `None`
    pub fn rsdp(self, address: Option<PhysicalAddress>) -> Self {
        Self {
            rsdp: address,
            ..self
        }
    }

    pub fn serial_console(self, console: Option<SerialConsole>) -> Self {
        Self {
            serial_console: console,
            ..self
        }
    }

    /// An empty text is the same as no command line
    pub fn command_line(self, text: &'a str) -> Self {
        Self {
            command_line: text,
            ..self
        }
    }

    /// Lays the structure, the memory regions and the command line out in
    /// `block`, whose first byte is at `block_base`, and returns the address
    /// of the structure, the one the kernel receives in its first argument
    pub fn write_into(
        self,
        block_base: PhysicalAddress,
        block: &mut [u8],
    ) -> Result<PhysicalAddress, BuildError> {
        if !block_base.is_aligned(8) {
            return Err(BuildError::BlockUnaligned);
        }
        let (kernel_image, kernel_virtual_base) = required(self.kernel_image, "kernel_image")?;
        let kernel_stack = required(self.kernel_stack, "kernel_stack")?;
        let page_tables = required(self.page_tables, "page_tables")?;
        let loader_data = required(self.loader_data, "loader_data")?;
        let regions = required(self.memory_regions, "memory_regions")?;
        let uefi_memory_map = required(self.uefi_memory_map, "uefi_memory_map")?;

        let entry_size = RawMemoryRegion::SIZE as usize;
        let regions_offset = RawBootInformation::SIZE as usize;
        let command_line_offset =
            regions_offset.saturating_add(regions.len().saturating_mul(entry_size));
        let needed = command_line_offset.saturating_add(self.command_line.len());
        if block.len() < needed {
            return Err(BuildError::BlockTooSmall { needed });
        }

        let raw = RawBootInformation {
            magic: RawBootInformation::MAGIC,
            version: RawBootInformation::VERSION,
            size: RawBootInformation::SIZE,
            physical_map_base: self.physical_map_base,
            kernel_image: RawRange::from(kernel_image),
            kernel_virtual_base,
            kernel_stack: RawRange::from(kernel_stack),
            page_tables: RawRange::from(page_tables),
            loader_data: RawRange::from(loader_data),
            memory_map: RawTable {
                base: block_base + regions_offset as u64,
                count: regions.len() as u64,
                entry_size: RawMemoryRegion::SIZE,
            },
            uefi_memory_map: RawTable::from(uefi_memory_map.table),
            uefi_descriptor_version: uefi_memory_map.descriptor_version,
            padding: 0,
            uefi_system_table: nonzero(self.uefi_system_table),
            rsdp: nonzero(self.rsdp),
            device_tree: RawRange::ABSENT,
            framebuffer: RawFramebuffer::ABSENT,
            command_line: RawRange {
                base: block_base + command_line_offset as u64,
                size: self.command_line.len() as u64,
            },
            initial_ramdisk: RawRange::ABSENT,
            serial_console: RawSerialConsole::from(self.serial_console),
        };
        raw.write_to(block);
        for (index, region) in regions.iter().enumerate() {
            RawMemoryRegion::from(*region).write_to(block, regions_offset + index * entry_size);
        }
        fields::put_bytes(block, command_line_offset, self.command_line.as_bytes());
        Ok(block_base)
    }
}

fn required<T>(value: Option<T>, name: &'static str) -> Result<T, BuildError> {
    value.ok_or(BuildError::MissingField(name))
}

fn nonzero(address: Option<PhysicalAddress>) -> Option<NonZeroU64> {
    address.and_then(|address| NonZeroU64::new(address.as_u64()))
}

#[cfg(test)]
mod tests {
    use std::vec;
    use std::vec::Vec;

    use address::AddressLayout;

    use super::*;
    use crate::information::testing::TestLayout;
    use crate::{BootInformation, MemoryKind, Table};

    const BLOCK_BASE: PhysicalAddress = PhysicalAddress::new(0x5000_0000);

    fn physical(base: u64, size: u64) -> PhysicalRange {
        PhysicalRange::new(PhysicalAddress::new(base), size).unwrap()
    }

    fn regions() -> [MemoryRegion; 2] {
        [
            MemoryRegion {
                range: physical(0x1000, 0x9f000),
                kind: MemoryKind::Usable,
            },
            MemoryRegion {
                range: physical(0x10_0000, 0x20_0000),
                kind: MemoryKind::KernelImage,
            },
        ]
    }

    fn uefi_memory_map() -> UefiMemoryMap {
        UefiMemoryMap {
            table: Table {
                base: PhysicalAddress::new(0x5000_1000),
                count: 40,
                entry_size: 48,
            },
            descriptor_version: 1,
        }
    }

    fn complete<'a>(regions: &'a [MemoryRegion]) -> BootInformationBuilder<'a> {
        BootInformationBuilder::new(TestLayout::PHYSICAL_MAP_BASE)
            .kernel_image(
                physical(0x10_0000, 0x20_0000),
                VirtualAddress::new(0xffff_ffff_8000_0000),
            )
            .kernel_stack(physical(0x30_0000, 0x1_0000))
            .page_tables(physical(0x40_0000, 0x10_0000))
            .loader_data(physical(0x5000_0000, 0x4000))
            .memory_regions(regions)
            .uefi_memory_map(uefi_memory_map())
    }

    #[test]
    fn every_field_round_trips() {
        let regions = regions();
        let mut block = vec![0u8; 0x1000];
        let structure = complete(&regions)
            .uefi_system_table(Some(PhysicalAddress::new(0x7f00_0000)))
            .rsdp(Some(PhysicalAddress::new(0x7e00_0000)))
            .serial_console(Some(SerialConsole::Pl011 {
                base: PhysicalAddress::new(0x0900_0000),
            }))
            .command_line("console=serial")
            .write_into(BLOCK_BASE, &mut block)
            .unwrap();
        assert_eq!(structure, BLOCK_BASE);

        let raw = RawBootInformation::read_from(&block).unwrap();
        let information = BootInformation::decode::<TestLayout>(&raw).unwrap();
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
        assert_eq!(information.page_tables(), physical(0x40_0000, 0x10_0000));
        assert_eq!(information.loader_data(), physical(0x5000_0000, 0x4000));
        assert_eq!(information.uefi_memory_map(), uefi_memory_map());
        assert_eq!(
            information.uefi_system_table(),
            Some(PhysicalAddress::new(0x7f00_0000))
        );
        assert_eq!(information.rsdp(), Some(PhysicalAddress::new(0x7e00_0000)));
        assert_eq!(
            information.serial_console(),
            Some(SerialConsole::Pl011 {
                base: PhysicalAddress::new(0x0900_0000)
            })
        );
        assert_eq!(information.device_tree(), None);
        assert_eq!(information.framebuffer(), None);
        assert_eq!(information.initial_ramdisk(), None);

        let table = information.memory_map();
        assert_eq!(table.base, BLOCK_BASE + 264);
        assert_eq!(table.count, 2);
        let table_bytes = &block[264..264 + table.size() as usize];
        let decoded: Vec<MemoryRegion> = information.memory_regions(table_bytes).collect();
        assert_eq!(decoded, regions);

        let command_line = information.command_line().unwrap();
        assert_eq!(command_line.base, BLOCK_BASE + 264 + 48);
        assert_eq!(command_line.size.get(), 14);
        assert_eq!(&block[312..326], b"console=serial");
    }

    #[test]
    fn absent_optional_fields_round_trip_as_none() {
        let regions = regions();
        let mut block = vec![0xaau8; 0x1000];
        complete(&regions)
            .write_into(BLOCK_BASE, &mut block)
            .unwrap();
        let raw = RawBootInformation::read_from(&block).unwrap();
        let information = BootInformation::decode::<TestLayout>(&raw).unwrap();
        assert_eq!(information.uefi_system_table(), None);
        assert_eq!(information.rsdp(), None);
        assert_eq!(information.serial_console(), None);
        assert_eq!(information.command_line(), None);
        assert_eq!(information.device_tree(), None);
        assert_eq!(information.framebuffer(), None);
        assert_eq!(information.initial_ramdisk(), None);
    }

    #[test]
    fn a_uart_port_round_trips_as_a_port_number() {
        let regions = regions();
        let mut block = vec![0u8; 0x1000];
        complete(&regions)
            .serial_console(Some(SerialConsole::Uart16550 { port: 0x3f8 }))
            .write_into(BLOCK_BASE, &mut block)
            .unwrap();
        let raw = RawBootInformation::read_from(&block).unwrap();
        let information = BootInformation::decode::<TestLayout>(&raw).unwrap();
        assert_eq!(
            information.serial_console(),
            Some(SerialConsole::Uart16550 { port: 0x3f8 })
        );
    }

    #[test]
    fn address_zero_is_written_as_absent() {
        let regions = regions();
        let mut block = vec![0u8; 0x1000];
        complete(&regions)
            .rsdp(Some(PhysicalAddress::new(0)))
            .write_into(BLOCK_BASE, &mut block)
            .unwrap();
        let raw = RawBootInformation::read_from(&block).unwrap();
        let information = BootInformation::decode::<TestLayout>(&raw).unwrap();
        assert_eq!(information.rsdp(), None);
    }

    #[test]
    fn block_too_small_names_the_need() {
        let regions = regions();
        let mut block = vec![0u8; 300];
        let result = complete(&regions)
            .command_line("0123456789")
            .write_into(BLOCK_BASE, &mut block);
        assert_eq!(result, Err(BuildError::BlockTooSmall { needed: 322 }));
    }

    #[test]
    fn a_missing_required_field_is_named() {
        let regions = regions();
        let mut block = vec![0u8; 0x1000];
        let result = BootInformationBuilder::new(TestLayout::PHYSICAL_MAP_BASE)
            .kernel_image(physical(0x10_0000, 0x20_0000), VirtualAddress::new(0))
            .memory_regions(&regions)
            .write_into(BLOCK_BASE, &mut block);
        assert_eq!(result, Err(BuildError::MissingField("kernel_stack")));
    }

    #[test]
    fn an_unaligned_block_is_rejected() {
        let regions = regions();
        let mut block = vec![0u8; 0x1000];
        let result = complete(&regions).write_into(PhysicalAddress::new(0x5000_0004), &mut block);
        assert_eq!(result, Err(BuildError::BlockUnaligned));
    }

    #[test]
    fn read_from_needs_the_whole_structure() {
        assert!(RawBootInformation::read_from(&[0u8; 263]).is_none());
        assert!(RawBootInformation::read_from(&[0u8; 264]).is_some());
    }
}
