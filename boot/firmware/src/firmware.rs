use crate::{
    Console, ConsoleError, FileError, Memory, MemoryMapSummary, MemoryRegion, Platform, Volume,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitError {
    /// The memory map kept changing between reading it and leaving, and the
    /// loader gave up retrying
    MapChangedTooOften,
    /// A buffer was too small or the firmware reported a failure
    Device,
}

/// Everything the boot sequence needs from the firmware
pub trait Firmware {
    type TextConsole: Console;
    type SerialConsole: Console;
    type Volume: Volume;
    type Memory: Memory;
    type Platform: Platform;

    fn text_console(&self) -> Result<Self::TextConsole, ConsoleError>;

    fn serial_console(&self) -> Result<Self::SerialConsole, ConsoleError>;

    /// The volume the loader was started from
    fn boot_volume(&self) -> Result<Self::Volume, FileError>;

    fn memory(&self) -> &Self::Memory;

    fn platform(&self) -> &Self::Platform;

    /// Takes the final memory map and leaves boot services; after this only
    /// the buffers filled here are valid. `raw` receives the firmware's own
    /// map, `regions` the converted one. Nothing may be allocated or printed
    /// between the map and the exit, which is why both happen in this one call
    fn exit_boot_services(
        self,
        raw: &mut [u8],
        regions: &mut [MemoryRegion],
    ) -> Result<MemoryMapSummary, ExitError>;
}
