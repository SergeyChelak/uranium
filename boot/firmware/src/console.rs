#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleError {
    /// The firmware offers no such console on this machine
    Unavailable,
    /// The console exists but the device reported a failure
    Device,
}

/// Text output the boot sequence can rely on without knowing the firmware
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
    use std::string::String;

    struct Recorder {
        written: String,
        fail_after: usize,
    }

    impl Console for Recorder {
        fn write_str(&mut self, text: &str) -> Result<(), ConsoleError> {
            if self.fail_after == 0 {
                return Err(ConsoleError::Device);
            }
            self.fail_after -= 1;
            self.written.push_str(text);
            Ok(())
        }
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
        assert_eq!(console.write_line("uranium"), Err(ConsoleError::Device));
        assert_eq!(console.written, "uranium");
    }
}
