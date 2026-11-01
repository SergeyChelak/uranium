use firmware::{Console, ConsoleError};
use uefi::SimpleTextOutputProtocol;

use crate::error::firmware_status;

pub struct TextConsole<'a> {
    protocol: &'a SimpleTextOutputProtocol,
}

impl<'a> TextConsole<'a> {
    pub(crate) fn new(protocol: &'a SimpleTextOutputProtocol) -> Self {
        Self { protocol }
    }
}

impl Console for TextConsole<'_> {
    fn write_str(&mut self, text: &str) -> Result<(), ConsoleError> {
        self.protocol
            .output_string(text)
            .map_err(|status| ConsoleError::Firmware(firmware_status(status)))
    }
}
