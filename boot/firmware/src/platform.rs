use address::PhysicalAddress;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformError {
    /// The firmware could not provide the bytes, or they do not fit the buffer
    Unreadable,
}

/// What the loader learns about the machine from the firmware besides files
/// and memory
pub trait Platform {
    /// The firmware's system table, handed on to the kernel
    fn system_table(&self) -> PhysicalAddress;

    /// The root of the ACPI tables, when the firmware installed one
    fn rsdp(&self) -> Option<PhysicalAddress>;

    /// Copies `buffer.len()` bytes of firmware-provided memory, such as ACPI
    /// tables, into `buffer`
    fn read(&self, address: PhysicalAddress, buffer: &mut [u8]) -> Result<(), PlatformError>;

    /// The image's load options as UTF-8 in `buffer`; the number of bytes
    /// written, zero when there are none
    fn load_options(&self, buffer: &mut [u8]) -> Result<usize, PlatformError>;
}
