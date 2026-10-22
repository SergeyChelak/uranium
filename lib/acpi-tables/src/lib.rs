//! Readers for the ACPI tables the loader follows: the root pointer, the
//! extended system description table and the serial port console redirection
//! table. Every parser works on bytes the caller took out of firmware memory,
//! and the lookup follows addresses only through a `TableReader` the caller
//! provides, so nothing here touches memory or assumes alignment.

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
    /// The bytes do not start with the signature of the expected table
    Signature,
    /// The bytes the table claims do not sum to zero
    Checksum,
    /// The table revision predates a field the loader needs
    Revision,
    /// The buffer is shorter than the table, or the length field is impossible
    Length,
    /// The address does not point at readable firmware memory
    Address,
    /// No table in the XSDT carries the requested signature
    NotFound,
}

impl From<byte_fields::OutOfBounds> for AcpiError {
    fn from(_: byte_fields::OutOfBounds) -> Self {
        Self::Length
    }
}
