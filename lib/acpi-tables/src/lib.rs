#![no_std]

#[cfg(test)]
extern crate std;

mod generic_address;
mod header;
mod lookup;
mod rsdp;
mod spcr;
mod xsdt;

pub use generic_address::{AddressSpace, GenericAddress};
pub use header::TableHeader;
pub use lookup::TableReader;
pub use rsdp::Rsdp;
pub use spcr::{SerialInterface, Spcr};
pub use xsdt::Xsdt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiError {
    Signature,
    Checksum,
    /// The table revision predates a field the loader needs
    Revision,
    /// The buffer is shorter than the table, or the length field is impossible
    Length,
    /// The address does not point at readable firmware memory
    Address,
    NotFound,
}

impl From<byte_fields::OutOfBounds> for AcpiError {
    fn from(_: byte_fields::OutOfBounds) -> Self {
        Self::Length
    }
}
