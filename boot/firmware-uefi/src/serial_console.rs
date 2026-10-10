use firmware::{Console, ConsoleError};
use uefi::{SerialIoProtocol, Status};

const LINE_ENDING: &[u8] = b"\r\n";

pub struct SerialConsole<'a> {
    protocol: &'a SerialIoProtocol,
}

impl<'a> SerialConsole<'a> {
    pub(crate) fn new(protocol: &'a SerialIoProtocol) -> Self {
        Self { protocol }
    }
}

impl Console for SerialConsole<'_> {
    fn write_str(&mut self, text: &str) -> Result<(), ConsoleError> {
        let protocol = self.protocol;
        with_line_endings(text, |bytes| {
            write_all(bytes, |remaining| protocol.write(remaining))
        })
    }
}

/// Feeds the text to `write` piece by piece, turning every bare line feed
/// into carriage return plus line feed, which is what a terminal expects
fn with_line_endings(
    text: &str,
    mut write: impl FnMut(&[u8]) -> Result<(), ConsoleError>,
) -> Result<(), ConsoleError> {
    for piece in text.split_inclusive('\n') {
        match piece.strip_suffix('\n') {
            Some(line) if !line.ends_with('\r') => {
                if !line.is_empty() {
                    write(line.as_bytes())?;
                }
                write(LINE_ENDING)?;
            }
            _ => write(piece.as_bytes())?,
        }
    }
    Ok(())
}

/// Repeats `write` until the device has taken every byte; no progress
/// counts as a device failure, so a stuck port cannot hang the caller
fn write_all(
    mut bytes: &[u8],
    mut write: impl FnMut(&[u8]) -> Result<usize, Status>,
) -> Result<(), ConsoleError> {
    while !bytes.is_empty() {
        match write(bytes) {
            Ok(0) | Err(_) => return Err(ConsoleError::Device),
            Ok(written) => bytes = &bytes[written.min(bytes.len())..],
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{with_line_endings, write_all};
    use firmware::ConsoleError;
    use std::vec::Vec;
    use uefi::Status;

    fn with_line_endings_of(text: &str) -> Vec<u8> {
        let mut output = Vec::new();
        with_line_endings(text, |bytes| {
            output.extend_from_slice(bytes);
            Ok(())
        })
        .unwrap();
        output
    }

    #[test]
    fn line_feed_becomes_carriage_return_line_feed() {
        assert_eq!(with_line_endings_of("a\nb\n"), b"a\r\nb\r\n");
        assert_eq!(with_line_endings_of("\n"), b"\r\n");
    }

    #[test]
    fn existing_carriage_return_is_kept() {
        assert_eq!(with_line_endings_of("a\r\nb"), b"a\r\nb");
    }

    #[test]
    fn text_without_line_feed_is_passed_through() {
        assert_eq!(with_line_endings_of("uranium"), b"uranium");
        assert_eq!(with_line_endings_of(""), b"");
    }

    #[test]
    fn write_errors_stop_the_line() {
        let mut calls = 0;
        let result = with_line_endings("a\nb", |_| {
            calls += 1;
            Err(ConsoleError::Device)
        });
        assert_eq!(result, Err(ConsoleError::Device));
        assert_eq!(calls, 1);
    }

    #[test]
    fn write_all_repeats_until_everything_is_taken() {
        let mut taken = Vec::new();
        let result = write_all(b"uranium", |bytes| {
            let count = bytes.len().min(3);
            taken.extend_from_slice(&bytes[..count]);
            Ok(count)
        });
        assert_eq!(result, Ok(()));
        assert_eq!(taken, b"uranium");
    }

    #[test]
    fn write_all_treats_no_progress_as_a_device_failure() {
        assert_eq!(write_all(b"x", |_| Ok(0)), Err(ConsoleError::Device));
    }

    #[test]
    fn write_all_reports_a_firmware_error() {
        assert_eq!(
            write_all(b"x", |_| Err(Status::TIMEOUT)),
            Err(ConsoleError::Device)
        );
    }
}
