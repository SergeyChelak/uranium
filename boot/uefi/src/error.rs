use crate::Status;

/// What an operation of this crate can report beyond the firmware's own
/// status: faults the wrapper finds itself, which no status code of the
/// firmware describes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UefiError {
    /// The firmware answered with an error status
    Firmware(Status),
    /// The path has more UCS-2 code units than `File::open` holds, or a
    /// character outside the basic plane
    PathTooLong,
    /// The firmware reported success but handed back a null pointer
    NullPointer,
    /// The memory map contradicts its own description: a descriptor narrower
    /// than the declared structure, a length that is not whole descriptors,
    /// or more bytes than the buffer holds
    MalformedMemoryMap,
}

impl From<Status> for UefiError {
    fn from(status: Status) -> Self {
        Self::Firmware(status)
    }
}

#[cfg(test)]
mod tests {
    use super::UefiError;
    use crate::Status;

    #[test]
    fn a_status_becomes_a_firmware_error() {
        assert_eq!(
            UefiError::from(Status::NOT_FOUND),
            UefiError::Firmware(Status::NOT_FOUND)
        );
    }
}
