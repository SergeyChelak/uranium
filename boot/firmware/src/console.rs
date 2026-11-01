use crate::FirmwareError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleError {
    Unavailable,
    Firmware(FirmwareError),
}

impl From<FirmwareError> for ConsoleError {
    fn from(error: FirmwareError) -> Self {
        Self::Firmware(error)
    }
}

pub trait Console {
    fn write_str(&mut self, text: &str) -> Result<(), ConsoleError>;

    fn write_line(&mut self, text: &str) -> Result<(), ConsoleError> {
        self.write_str(text)?;
        self.write_str("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::{Console, ConsoleError};
    use crate::FirmwareError;
    use std::string::String;

    const FAILURE: ConsoleError = ConsoleError::Firmware(FirmwareError::NoProgress);

    struct Recorder {
        written: String,
        fail_after: usize,
    }

    impl Console for Recorder {
        fn write_str(&mut self, text: &str) -> Result<(), ConsoleError> {
            if self.fail_after == 0 {
                return Err(FAILURE);
            }
            self.fail_after -= 1;
            self.written.push_str(text);
            Ok(())
        }
    }

    #[test]
    fn a_firmware_fault_is_a_console_error() {
        assert_eq!(ConsoleError::from(FirmwareError::NoProgress), FAILURE);
    }

    #[test]
    fn write_line_appends_a_newline() {
        let mut console = Recorder {
            written: String::new(),
            fail_after: usize::MAX,
        };
        assert_eq!(console.write_line("uranium"), Ok(()));
        assert_eq!(console.written, "uranium\n");
    }

    #[test]
    fn write_line_stops_at_the_first_error() {
        let mut console = Recorder {
            written: String::new(),
            fail_after: 1,
        };
        assert_eq!(console.write_line("uranium"), Err(FAILURE));
        assert_eq!(console.written, "uranium");
    }
}
