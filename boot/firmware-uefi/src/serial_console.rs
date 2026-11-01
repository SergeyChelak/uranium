use firmware::{Console, ConsoleError, FirmwareError};
use uefi::{SerialIoProtocol, Status};

use crate::error::firmware_status;

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

/// No progress is an error, so a stuck port cannot hang the caller
fn write_all(
    mut bytes: &[u8],
    mut write: impl FnMut(&[u8]) -> Result<usize, Status>,
) -> Result<(), ConsoleError> {
    while !bytes.is_empty() {
        let written = write(bytes).map_err(firmware_status)?;
        if written == 0 {
            return Err(FirmwareError::NoProgress.into());
        }
        bytes = bytes
            .get(written..)
            .ok_or(FirmwareError::CountBeyondBuffer)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{with_line_endings, write_all};
    use crate::error::firmware_status;
    use firmware::{ConsoleError, FirmwareError};
    use std::vec::Vec;
    use uefi::Status;

    const FAILURE: ConsoleError = ConsoleError::Firmware(FirmwareError::NoProgress);

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
            Err(FAILURE)
        });
        assert_eq!(result, Err(FAILURE));
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
    fn write_all_treats_no_progress_as_a_firmware_fault() {
        assert_eq!(
            write_all(b"x", |_| Ok(0)),
            Err(ConsoleError::Firmware(FirmwareError::NoProgress))
        );
    }

    #[test]
    fn write_all_reports_the_firmware_status() {
        assert_eq!(
            write_all(b"x", |_| Err(Status::TIMEOUT)),
            Err(ConsoleError::Firmware(firmware_status(Status::TIMEOUT)))
        );
    }

    #[test]
    fn write_all_rejects_a_count_beyond_the_buffer() {
        assert_eq!(
            write_all(b"xy", |bytes| Ok(bytes.len() + 1)),
            Err(ConsoleError::Firmware(FirmwareError::CountBeyondBuffer))
        );
    }
}
