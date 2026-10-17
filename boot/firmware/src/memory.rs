use address::PhysicalAddress;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    /// The firmware has no run of free pages that long
    OutOfMemory,
    /// The firmware reported a failure
    Device,
}

/// Pages the firmware gave the loader: their physical base and a view of the
/// same bytes through the firmware's identity mapping, valid as long as the
/// firmware is
pub struct Allocation<'a> {
    pub base: PhysicalAddress,
    pub bytes: &'a mut [u8],
}

pub trait Memory {
    /// `page_count` contiguous pages of 4 KiB, wherever the firmware chooses
    fn allocate(&self, page_count: u64) -> Result<Allocation<'_>, MemoryError>;

    /// The buffer size the firmware's memory map needs right now, with no
    /// slack added
    fn memory_map_size(&self) -> Result<usize, MemoryError>;
}

/// What a region of physical memory may be used for: the firmware's picture
/// reduced to what the kernel must know
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    /// Free for the kernel once the loader is gone
    Usable,
    /// Never to be touched
    Reserved,
    /// ACPI tables; free once the kernel has read them
    AcpiReclaimable,
    /// ACPI non-volatile storage; preserved across sleep states
    AcpiNonVolatile,
    /// Device registers mapped into memory
    MemoryMappedIo,
    /// Code and data of the firmware's runtime services; preserved
    UefiRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    pub start: PhysicalAddress,
    pub page_count: u64,
    pub kind: MemoryKind,
}

/// What leaving boot services leaves behind: how much of each buffer was
/// filled and how the raw map is laid out
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMapSummary {
    pub region_count: usize,
    pub raw_length: usize,
    pub raw_descriptor_size: usize,
    pub raw_descriptor_version: u32,
}
