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

/// The byte order of a pixel in the framebuffer; the discriminant is the
/// code that travels in the wire format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Unknown = 0,
    Rgb8 = 1,
    Bgr8 = 2,
}

impl From<PixelFormat> for u32 {
    fn from(format: PixelFormat) -> u32 {
        format as u32
    }
}

impl From<u32> for PixelFormat {
    fn from(code: u32) -> Self {
        match code {
            1 => Self::Rgb8,
            2 => Self::Bgr8,
            _ => Self::Unknown,
        }
    }
}

/// The chip behind the serial port the loader found for the kernel's early
/// console; the discriminant is the code that travels in the wire format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerialKind {
    Uart16550 = 1,
    Pl011 = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownSerialKind(pub u32);

impl From<SerialKind> for u32 {
    fn from(kind: SerialKind) -> u32 {
        kind as u32
    }
}

impl TryFrom<u32> for SerialKind {
    type Error = UnknownSerialKind;

    fn try_from(code: u32) -> Result<Self, UnknownSerialKind> {
        match code {
            1 => Ok(Self::Uart16550),
            2 => Ok(Self::Pl011),
            other => Err(UnknownSerialKind(other)),
        }
    }
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
    fn every_kind_round_trips_through_u32() {
        for kind in MEMORY_KINDS {
            assert_eq!(MemoryKind::from(u32::from(kind)), kind);
        }
        for format in [PixelFormat::Unknown, PixelFormat::Rgb8, PixelFormat::Bgr8] {
            assert_eq!(PixelFormat::from(u32::from(format)), format);
        }
        for kind in [SerialKind::Uart16550, SerialKind::Pl011] {
            assert_eq!(SerialKind::try_from(u32::from(kind)), Ok(kind));
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

    #[test]
    fn unknown_pixel_format_is_unknown() {
        for code in [3, u32::MAX] {
            assert_eq!(PixelFormat::from(code), PixelFormat::Unknown);
        }
    }

    #[test]
    fn unknown_serial_kind_names_the_code() {
        for code in [0, 3, u32::MAX] {
            assert_eq!(SerialKind::try_from(code), Err(UnknownSerialKind(code)));
        }
    }
}
