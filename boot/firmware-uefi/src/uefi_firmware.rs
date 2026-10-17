use firmware::{ConsoleError, FileError};
use uefi::{
    Handle, LoadedImageProtocol, ResetKind, SerialIoProtocol, SimpleFileSystemProtocol, Status,
    SystemTable,
};

use crate::{SerialConsole, TextConsole, UefiVolume};

/// The firmware as the boot sequence sees it, reached through UEFI
pub struct UefiFirmware<'a> {
    image: Handle,
    system_table: &'a SystemTable,
}

impl<'a> UefiFirmware<'a> {
    /// `image` is the handle of the loader itself, the one the firmware
    /// passed to `efi_main`
    pub fn new(image: Handle, system_table: &'a SystemTable) -> Self {
        Self {
            image,
            system_table,
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

    /// The file system on the device the firmware loaded this image from
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

    /// Powers the machine off; temporary for M0, where no kernel takes over
    pub fn shutdown(self) -> ! {
        self.system_table
            .runtime_services()
            .reset_system(ResetKind::Shutdown, Status::SUCCESS)
    }
}
