use firmware::{FileError, Volume};
use uefi::Status;

pub struct UefiVolume {
    root: uefi::File,
}

impl UefiVolume {
    pub(crate) fn new(root: uefi::File) -> Self {
        Self { root }
    }
}

impl Volume for UefiVolume {
    type File = UefiFile;

    fn open(&self, path: &str) -> Result<UefiFile, FileError> {
        let file = self.root.open(path).map_err(open_error)?;
        let size = file.size().map_err(|_| FileError::Device)?;
        Ok(UefiFile { file, size })
    }
}

pub struct UefiFile {
    file: uefi::File,
    size: u64,
}

impl firmware::File for UefiFile {
    fn size(&self) -> u64 {
        self.size
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, FileError> {
        self.file.read(buffer).map_err(|_| FileError::Device)
    }
}

// `uefi::File::open` answers `INVALID_PARAMETER` for a path it could not encode
fn open_error(status: Status) -> FileError {
    match status {
        Status::NOT_FOUND => FileError::NotFound,
        Status::INVALID_PARAMETER => FileError::PathTooLong,
        _ => FileError::Device,
    }
}

#[cfg(test)]
mod tests {
    use super::open_error;
    use firmware::FileError;
    use uefi::Status;

    #[test]
    fn not_found_means_the_file_is_missing() {
        assert_eq!(open_error(Status::NOT_FOUND), FileError::NotFound);
    }

    #[test]
    fn invalid_parameter_means_the_path_was_rejected() {
        assert_eq!(
            open_error(Status::INVALID_PARAMETER),
            FileError::PathTooLong
        );
    }

    #[test]
    fn every_other_status_is_a_device_failure() {
        assert_eq!(open_error(Status::DEVICE_ERROR), FileError::Device);
        assert_eq!(open_error(Status::UNSUPPORTED), FileError::Device);
        assert_eq!(open_error(Status::OUT_OF_RESOURCES), FileError::Device);
    }
}
