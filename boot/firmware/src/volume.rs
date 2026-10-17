#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileError {
    /// The volume has no file of that name
    NotFound,
    /// The path is longer than the firmware accepts
    PathTooLong,
    /// The file ended before the buffer was full
    Short,
    /// The volume or the device behind it reported a failure
    Device,
}

/// The volume the loader was started from, read-only
pub trait Volume {
    type File: File;

    /// Opens `path` for reading; UEFI form, with `\` between components
    fn open(&self, path: &str) -> Result<Self::File, FileError>;
}

/// An open file; its size is known from the moment it was opened
pub trait File {
    fn size(&self) -> u64;

    /// Reads at the current position; `Ok(0)` at the end
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, FileError>;

    /// Fills the whole buffer or fails; `Short` when the file ends first
    fn read_exact(&mut self, mut buffer: &mut [u8]) -> Result<(), FileError> {
        while !buffer.is_empty() {
            let read = self.read(buffer)?;
            if read == 0 {
                return Err(FileError::Short);
            }
            buffer = buffer.get_mut(read..).ok_or(FileError::Device)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{File, FileError};

    /// A file whose firmware hands out at most `chunk` bytes per read
    struct ChunkedFile {
        content: &'static [u8],
        position: usize,
        chunk: usize,
    }

    impl File for ChunkedFile {
        fn size(&self) -> u64 {
            self.content.len() as u64
        }

        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, FileError> {
            let remaining = &self.content[self.position..];
            let count = remaining.len().min(buffer.len()).min(self.chunk);
            buffer[..count].copy_from_slice(&remaining[..count]);
            self.position += count;
            Ok(count)
        }
    }

    struct FailingFile;

    impl File for FailingFile {
        fn size(&self) -> u64 {
            0
        }

        fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, FileError> {
            Err(FileError::Device)
        }
    }

    #[test]
    fn read_exact_fills_the_buffer() {
        let mut file = ChunkedFile {
            content: b"uranium",
            position: 0,
            chunk: 3,
        };
        let mut buffer = [0; 7];
        assert_eq!(file.read_exact(&mut buffer), Ok(()));
        assert_eq!(&buffer, b"uranium");
    }

    #[test]
    fn read_exact_reports_a_short_file() {
        let mut file = ChunkedFile {
            content: b"ura",
            position: 0,
            chunk: 3,
        };
        let mut buffer = [0; 7];
        assert_eq!(file.read_exact(&mut buffer), Err(FileError::Short));
        assert_eq!(&buffer[..3], b"ura");
    }

    #[test]
    fn read_exact_stops_at_a_device_error() {
        let mut buffer = [0; 4];
        assert_eq!(FailingFile.read_exact(&mut buffer), Err(FileError::Device));
    }
}
