use firmware::{FirmwareError, FirmwareStatus};
use uefi::{ProtocolError, Status};

pub(crate) fn firmware_status(status: Status) -> FirmwareError {
    FirmwareError::Status(FirmwareStatus::new(status.raw()))
}

pub(crate) fn protocol_error(error: ProtocolError) -> FirmwareError {
    match error {
        ProtocolError::Firmware(status) => firmware_status(status),
        ProtocolError::NullPointer => FirmwareError::NullPointer,
    }
}

#[cfg(test)]
mod tests {
    use super::{firmware_status, protocol_error};
    use firmware::{FirmwareError, FirmwareStatus};
    use uefi::{ProtocolError, Status};

    #[test]
    fn a_status_keeps_its_raw_value() {
        assert_eq!(
            firmware_status(Status::NOT_FOUND),
            FirmwareError::Status(FirmwareStatus::new(Status::NOT_FOUND.raw()))
        );
    }

    #[test]
    fn a_protocol_error_keeps_its_cause() {
        assert_eq!(
            protocol_error(ProtocolError::Firmware(Status::UNSUPPORTED)),
            firmware_status(Status::UNSUPPORTED)
        );
        assert_eq!(
            protocol_error(ProtocolError::NullPointer),
            FirmwareError::NullPointer
        );
    }
}
