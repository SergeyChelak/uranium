/// Opaque to the boot sequence: reported, never interpreted
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareError {
    Status(FirmwareStatus),
    NullPointer,
    CountBeyondBuffer,
    NoProgress,
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
