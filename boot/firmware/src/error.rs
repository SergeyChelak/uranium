/// The firmware's own error code, as wide as its native word; the boot
/// sequence cannot interpret it, only report it
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmwareStatus(usize);

impl FirmwareStatus {
    pub const fn new(raw: usize) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> usize {
        self.0
    }
}

/// How a call into the firmware failed: with its own status, or with an
/// answer that breaks the contract of the call
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareError {
    /// The firmware answered with an error status
    Status(FirmwareStatus),
    /// The firmware reported success but handed back a null pointer
    NullPointer,
    /// The firmware reported more bytes than the buffer it was given holds
    CountBeyondBuffer,
    /// The firmware reported success but took none of the bytes
    NoProgress,
    /// The firmware's memory map contradicts its own description
    MalformedMemoryMap,
}

#[cfg(test)]
mod tests {
    use super::FirmwareStatus;

    #[test]
    fn a_status_keeps_its_raw_value() {
        assert_eq!(
            FirmwareStatus::new(0x8000_0000_0000_0007).raw(),
            0x8000_0000_0000_0007
        );
    }
}
