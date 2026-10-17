use firmware::{ConsoleError, ExitError, FileError, MemoryMapSummary, MemoryRegion};
use uefi::{
    Handle, LoadedImageProtocol, ResetKind, SerialIoProtocol, SimpleFileSystemProtocol, Status,
    SystemTable,
};

use crate::{SerialConsole, TextConsole, UefiMemory, UefiPlatform, UefiVolume, exit};

/// The firmware as the boot sequence sees it, reached through UEFI
pub struct UefiFirmware<'a> {
    image: Handle,
    system_table: &'a SystemTable,
    memory: UefiMemory<'a>,
    platform: UefiPlatform<'a>,
}

impl<'a> UefiFirmware<'a> {
    pub fn new(image: Handle, system_table: &'a SystemTable) -> Self {
        Self {
            image,
            system_table,
            memory: UefiMemory::new(system_table.boot_services()),
            platform: UefiPlatform::new(image, system_table),
        }
    }

    pub fn text_console(&self) -> Result<TextConsole<'a>, ConsoleError> {
        self.system_table
            .console_out()
            .map(TextConsole::new)
            .ok_or(ConsoleError::Unavailable)
    }

    pub fn serial_console(&self) -> Result<SerialConsole<'a>, ConsoleError> {
        match self
            .system_table
            .boot_services()
            .locate_protocol::<SerialIoProtocol>()
        {
            Ok(protocol) => Ok(SerialConsole::new(protocol)),
            Err(Status::NOT_FOUND) => Err(ConsoleError::Unavailable),
            Err(_) => Err(ConsoleError::Device),
        }
    }

    pub fn boot_volume(&self) -> Result<UefiVolume, FileError> {
        let boot_services = self.system_table.boot_services();
        let image = boot_services
            .handle_protocol::<LoadedImageProtocol>(self.image)
            .map_err(|_| FileError::Device)?;
        let device = image.device_handle().ok_or(FileError::Device)?;
        let file_system = boot_services
            .handle_protocol::<SimpleFileSystemProtocol>(device)
            .map_err(|_| FileError::Device)?;
        let root = file_system.open_volume().map_err(|_| FileError::Device)?;
        Ok(UefiVolume::new(root))
    }

    pub fn memory(&self) -> &UefiMemory<'a> {
        &self.memory
    }

    pub fn platform(&self) -> &UefiPlatform<'a> {
        &self.platform
    }

    pub fn exit_boot_services(
        self,
        raw: &mut [u8],
        regions: &mut [MemoryRegion],
    ) -> Result<MemoryMapSummary, ExitError> {
        exit::exit_boot_services(self.system_table.boot_services(), self.image, raw, regions)
    }

    /// Powers the machine off; temporary for M0, where no kernel takes over
    pub fn shutdown(self) -> ! {
        self.system_table
            .runtime_services()
            .reset_system(ResetKind::Shutdown, Status::SUCCESS)
    }
}
