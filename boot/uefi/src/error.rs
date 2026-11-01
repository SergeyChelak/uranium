use crate::Status;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    Firmware(Status),
    NullPointer,
}

impl From<Status> for ProtocolError {
    fn from(status: Status) -> Self {
        Self::Firmware(status)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenError {
    /// The path has more UCS-2 code units than `File::open` holds, or a
    /// character outside the basic plane
    PathTooLong,
    Protocol(ProtocolError),
}

impl From<ProtocolError> for OpenError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<Status> for OpenError {
    fn from(status: Status) -> Self {
        Self::Protocol(ProtocolError::Firmware(status))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMapError {
    Firmware(Status),
    /// The map contradicts its own description: a descriptor narrower than
    /// the declared structure, a length that is not whole descriptors, or
    /// more bytes than the buffer holds
    Malformed,
}

impl From<Status> for MemoryMapError {
    fn from(status: Status) -> Self {
        Self::Firmware(status)
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryMapError, OpenError, ProtocolError};
    use crate::Status;

    #[test]
    fn a_status_is_the_firmware_refusing() {
        assert_eq!(
            ProtocolError::from(Status::NOT_FOUND),
            ProtocolError::Firmware(Status::NOT_FOUND)
        );
        assert_eq!(
            MemoryMapError::from(Status::BUFFER_TOO_SMALL),
            MemoryMapError::Firmware(Status::BUFFER_TOO_SMALL)
        );
    }

    #[test]
    fn an_open_fails_the_way_a_protocol_does() {
        assert_eq!(
            OpenError::from(ProtocolError::NullPointer),
            OpenError::Protocol(ProtocolError::NullPointer)
        );
        assert_eq!(
            OpenError::from(Status::NOT_FOUND),
            OpenError::Protocol(ProtocolError::Firmware(Status::NOT_FOUND))
        );
    }
}
