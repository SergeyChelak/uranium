use firmware::{Console, ConsoleError};
use uefi::SimpleTextOutputProtocol;

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
            .write_str(text)
            .map_err(|_| ConsoleError::Device)
    }
}
