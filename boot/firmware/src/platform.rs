use acpi_tables::{AcpiError, Spcr};
use address::PhysicalAddress;

use crate::FirmwareError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformError {
    /// The firmware has no ACPI tables, or none with the requested signature
    MissingTable,
    /// The ACPI tables are there but broken on the way to the requested one
    Acpi(AcpiError),
    /// The load options do not fit the buffer
    LoadOptionsTooLong,
    /// The firmware failed the call
    Firmware(FirmwareError),
}

impl From<FirmwareError> for PlatformError {
    fn from(error: FirmwareError) -> Self {
        Self::Firmware(error)
    }
}

impl From<AcpiError> for PlatformError {
    fn from(error: AcpiError) -> Self {
        match error {
            AcpiError::NotFound => Self::MissingTable,
            other => Self::Acpi(other),
        }
    }
}

/// What the loader learns about the machine from the firmware besides files
/// and memory
pub trait Platform {
    /// The firmware's system table, handed on to the kernel
    fn system_table_address(&self) -> PhysicalAddress;

    /// The root of the ACPI tables, when the firmware installed one
    fn rsdp_address(&self) -> Option<PhysicalAddress>;

    /// The serial port console redirection table, parsed and verified
    fn spcr(&self) -> Result<Spcr, PlatformError>;

    /// The image's load options as UTF-8 in `buffer`; the number of bytes
    /// written, zero when there are none
    fn load_options(&self, buffer: &mut [u8]) -> Result<usize, PlatformError>;
}

#[cfg(test)]
mod tests {
    use acpi_tables::AcpiError;

    use super::PlatformError;
    use crate::FirmwareError;

    #[test]
    fn a_firmware_fault_is_a_platform_error() {
        assert_eq!(
            PlatformError::from(FirmwareError::NullPointer),
            PlatformError::Firmware(FirmwareError::NullPointer)
        );
    }

    #[test]
    fn a_table_nobody_published_is_missing() {
        assert_eq!(
            PlatformError::from(AcpiError::NotFound),
            PlatformError::MissingTable
        );
    }

    #[test]
    fn a_broken_chain_keeps_its_cause() {
        assert_eq!(
            PlatformError::from(AcpiError::Checksum),
            PlatformError::Acpi(AcpiError::Checksum)
        );
    }
}
