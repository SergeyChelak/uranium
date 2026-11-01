use firmware::{FileError, Volume};
use uefi::{Status, UefiError};

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
        let size = file.size().map_err(|_| FileError::Device)?;
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
        self.file.read(buffer).map_err(|_| FileError::Device)
    }
}

fn open_error(error: UefiError) -> FileError {
    match error {
        UefiError::Firmware(Status::NOT_FOUND) => FileError::NotFound,
        UefiError::PathTooLong => FileError::PathTooLong,
        _ => FileError::Device,
    }
}

#[cfg(test)]
mod tests {
    use super::open_error;
    use firmware::FileError;
    use uefi::{Status, UefiError};

    #[test]
    fn not_found_means_the_file_is_missing() {
        assert_eq!(
            open_error(UefiError::Firmware(Status::NOT_FOUND)),
            FileError::NotFound
        );
    }

    #[test]
    fn a_rejected_path_is_too_long() {
        assert_eq!(open_error(UefiError::PathTooLong), FileError::PathTooLong);
    }

    #[test]
    fn every_other_error_is_a_device_failure() {
        assert_eq!(
            open_error(UefiError::Firmware(Status::DEVICE_ERROR)),
            FileError::Device
        );
        assert_eq!(
            open_error(UefiError::Firmware(Status::UNSUPPORTED)),
            FileError::Device
        );
        assert_eq!(open_error(UefiError::NullPointer), FileError::Device);
    }
}
