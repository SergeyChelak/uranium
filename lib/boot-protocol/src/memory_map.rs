use address::PhysicalAddress;
use byte_fields::{Decode, Decoder, Encode, Encoder, OutOfBounds, layout};

use crate::{PhysicalRange, Table};

#[repr(C)]
pub(crate) struct RawMemoryRegion {
    pub(crate) base: PhysicalAddress,
    pub(crate) size: u64,
    pub(crate) kind: u32,
    pub(crate) padding: u32,
}

impl RawMemoryRegion {
    pub(crate) const SIZE: u64 = 24;
}

impl Encode for RawMemoryRegion {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), OutOfBounds> {
        encoder.write(&self.base)?;
        encoder.write(&self.size)?;
        encoder.write(&self.kind)?;
        encoder.write(&self.padding)
    }
}

impl Decode for RawMemoryRegion {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, OutOfBounds> {
        Ok(Self {
            base: decoder.read()?,
            size: decoder.read()?,
            kind: decoder.read()?,
            padding: decoder.read()?,
        })
    }
}

impl From<MemoryRegion> for RawMemoryRegion {
    fn from(region: MemoryRegion) -> Self {
        Self {
            base: region.range.base,
            size: region.range.size.get(),
            kind: u32::from(region.kind),
            padding: 0,
        }
    }
}

layout!(RawMemoryRegion, RawMemoryRegion::SIZE as usize, {
    base: 0,
    size: 8,
    kind: 16,
    padding: 20,
});

/// How the kernel may use a region of the memory map; the discriminant is
/// the code that travels in the wire format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    Usable = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    AcpiNonVolatile = 4,
    MemoryMappedIo = 5,
    UefiRuntime = 6,
    KernelImage = 7,
    KernelStack = 8,
    PageTables = 9,
    LoaderData = 10,
    Framebuffer = 11,
    InitialRamdisk = 12,
}

impl From<MemoryKind> for u32 {
    fn from(kind: MemoryKind) -> u32 {
        kind as u32
    }
}

impl From<u32> for MemoryKind {
    /// A code this kernel does not know names memory it must not touch
    fn from(code: u32) -> Self {
        match code {
            1 => Self::Usable,
            3 => Self::AcpiReclaimable,
            4 => Self::AcpiNonVolatile,
            5 => Self::MemoryMappedIo,
            6 => Self::UefiRuntime,
            7 => Self::KernelImage,
            8 => Self::KernelStack,
            9 => Self::PageTables,
            10 => Self::LoaderData,
            11 => Self::Framebuffer,
            12 => Self::InitialRamdisk,
            _ => Self::Reserved,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    pub range: PhysicalRange,
    pub kind: MemoryKind,
}

impl MemoryRegion {
    /// `None` for an entry without memory behind it
    fn decode(entry: &[u8]) -> Option<Self> {
        let raw = RawMemoryRegion::decode(&mut Decoder::new(entry)).ok()?;
        let range = PhysicalRange::new(raw.base, raw.size)?;
        Some(Self {
            range,
            kind: MemoryKind::from(raw.kind),
        })
    }
}

/// The regions of a table whose bytes the caller read, stepping by the
/// entry size the loader wrote so that longer entries of a later version
/// decode the same
pub(crate) fn regions(
    table: &[u8],
    count: u64,
    entry_size: usize,
) -> impl Iterator<Item = MemoryRegion> + '_ {
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    table
        .chunks_exact(entry_size)
        .take(count)
        .filter_map(MemoryRegion::decode)
}

/// The memory map as the firmware gave it, kept for the kernel to hand back
/// through `SetVirtualAddressMap`; this crate does not read its entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UefiMemoryMap {
    pub table: Table,
    pub descriptor_version: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMORY_KINDS: [MemoryKind; 12] = [
        MemoryKind::Usable,
        MemoryKind::Reserved,
        MemoryKind::AcpiReclaimable,
        MemoryKind::AcpiNonVolatile,
        MemoryKind::MemoryMappedIo,
        MemoryKind::UefiRuntime,
        MemoryKind::KernelImage,
        MemoryKind::KernelStack,
        MemoryKind::PageTables,
        MemoryKind::LoaderData,
        MemoryKind::Framebuffer,
        MemoryKind::InitialRamdisk,
    ];

    #[test]
    fn every_memory_kind_round_trips_through_u32() {
        for kind in MEMORY_KINDS {
            assert_eq!(MemoryKind::from(u32::from(kind)), kind);
        }
    }

    #[test]
    fn memory_kind_codes_are_consecutive_from_one() {
        let codes = MEMORY_KINDS.map(u32::from);
        assert_eq!(codes, core::array::from_fn(|index| index as u32 + 1));
    }

    #[test]
    fn unknown_memory_kind_is_reserved() {
        for code in [0, 13, u32::MAX] {
            assert_eq!(MemoryKind::from(code), MemoryKind::Reserved);
        }
    }
}
