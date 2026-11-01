use firmware::{FileError, Volume};
use uefi::{OpenError, ProtocolError, Status};

use crate::error::{firmware_status, protocol_error};

pub struct UefiVolume<'b> {
    root: uefi::File<'b>,
}

impl<'b> UefiVolume<'b> {
    pub(crate) fn new(root: uefi::File<'b>) -> Self {
        Self { root }
    }
}

impl<'b> Volume for UefiVolume<'b> {
    type File = UefiFile<'b>;

    fn open(&self, path: &str) -> Result<UefiFile<'b>, FileError> {
        let file = self.root.open(path).map_err(open_error)?;
        let size = file.size().map_err(firmware_status)?;
        Ok(UefiFile { file, size })
    }
}

pub struct UefiFile<'b> {
    file: uefi::File<'b>,
    size: u64,
}

impl firmware::File for UefiFile<'_> {
    fn size(&self) -> u64 {
        self.size
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, FileError> {
        Ok(self.file.read(buffer).map_err(firmware_status)?)
    }
}

fn open_error(error: OpenError) -> FileError {
    match error {
        OpenError::Protocol(ProtocolError::Firmware(Status::NOT_FOUND)) => FileError::NotFound,
        OpenError::PathTooLong => FileError::PathTooLong,
        OpenError::Protocol(other) => FileError::Firmware(protocol_error(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::open_error;
    use crate::error::firmware_status;
    use firmware::{FileError, FirmwareError};
    use uefi::{OpenError, ProtocolError, Status};

    #[test]
    fn not_found_means_the_file_is_missing() {
        assert_eq!(
            open_error(OpenError::Protocol(ProtocolError::Firmware(
                Status::NOT_FOUND
            ))),
            FileError::NotFound
        );
    }

    #[test]
    fn a_rejected_path_is_too_long() {
        assert_eq!(open_error(OpenError::PathTooLong), FileError::PathTooLong);
    }

    #[test]
    fn every_other_error_comes_from_the_firmware() {
        assert_eq!(
            open_error(OpenError::Protocol(ProtocolError::Firmware(
                Status::DEVICE_ERROR
            ))),
            FileError::Firmware(firmware_status(Status::DEVICE_ERROR))
        );
        assert_eq!(
            open_error(OpenError::Protocol(ProtocolError::NullPointer)),
            FileError::Firmware(FirmwareError::NullPointer)
        );
    }
}
