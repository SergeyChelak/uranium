//! Readers for the ACPI tables the loader follows: the root pointer, the
//! extended system description table and the serial port console redirection
//! table. Every reader works on a byte copy the caller took out of firmware
//! memory, so nothing here touches an address or assumes alignment.

#![no_std]

#[cfg(test)]
extern crate std;

mod fields;
mod generic_address;
mod header;
mod rsdp;
mod spcr;
mod xsdt;

pub use generic_address::{AddressSpace, GenericAddress};
pub use header::TableHeader;
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
}
