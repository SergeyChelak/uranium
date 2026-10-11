use firmware::ConsoleError;
use uefi::{ResetKind, SerialIoProtocol, Status, SystemTable};

use crate::{SerialConsole, TextConsole};

/// The firmware as the boot sequence sees it, reached through UEFI
pub struct UefiFirmware<'a> {
    system_table: &'a SystemTable,
}

impl<'a> UefiFirmware<'a> {
    pub fn new(system_table: &'a SystemTable) -> Self {
        Self { system_table }
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

    /// Powers the machine off; temporary for M0, where no kernel takes over
    pub fn shutdown(self) -> ! {
        self.system_table
            .runtime_services()
            .reset_system(ResetKind::Shutdown, Status::SUCCESS)
    }
}
