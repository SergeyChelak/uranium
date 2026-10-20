//! The Serial Port Console Redirection table (SPCR): the ACPI table, defined
//! by Microsoft, that describes the serial port meant for a console

use crate::{AcpiError, GenericAddress, TableHeader};

// Microsoft SPCR 1.10, table layout
const SIGNATURE: &[u8; 4] = b"SPCR";
const INTERFACE_TYPE_OFFSET: usize = 36;
const BASE_ADDRESS_OFFSET: usize = 40;
// Microsoft SPCR 1.10: revision 1 names the interface itself, revision 2 and
// above use the serial port subtypes of DBG2
const DBG2_SUBTYPES_REVISION: u8 = 2;
// Microsoft SPCR 1.10, revision 1: full 16550; Microsoft DBG2, Table 3:
// fully 16550-compatible
const FULL_16550: u8 = 0;
// Microsoft DBG2, Table 3: Arm PL011 UART
const ARM_PL011: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerialInterface {
    Uart16550,
    Pl011,
    Other(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spcr {
    pub interface: SerialInterface,
    /// `None` when the table says console redirection is disabled
    pub base: Option<GenericAddress>,
}

impl Spcr {
    /// The fields after the base address describe settings the kernel's own
    /// driver decides, so they are not read
    pub fn parse(bytes: &[u8]) -> Result<Self, AcpiError> {
        let (header, table) = TableHeader::verify(bytes, SIGNATURE)?;
        let interface = interface_of(header.revision, table.u8(INTERFACE_TYPE_OFFSET)?);
        let base = GenericAddress::parse(table.tail(BASE_ADDRESS_OFFSET)?)?;
        Ok(Self {
            interface,
            base: redirection_target(base),
        })
    }
}

fn interface_of(revision: u8, code: u8) -> SerialInterface {
    match code {
        FULL_16550 => SerialInterface::Uart16550,
        ARM_PL011 if revision >= DBG2_SUBTYPES_REVISION => SerialInterface::Pl011,
        other => SerialInterface::Other(other),
    }
}

// Microsoft SPCR 1.10: a base address of 0 means redirection is disabled
fn redirection_target(base: GenericAddress) -> Option<GenericAddress> {
    (base.raw_value != 0).then_some(base)
}

#[cfg(test)]
mod tests {
    use std::vec;
    use std::vec::Vec;

    use super::{SerialInterface, Spcr};
    use crate::header::testing::table;
    use crate::{AcpiError, AddressSpace, GenericAddress};

    fn spcr(revision: u8, interface: u8, space: u8, raw_value: u64) -> Vec<u8> {
        let mut body = vec![0u8; 16];
        body[0] = interface;
        body[4] = space;
        body[8..16].copy_from_slice(&raw_value.to_le_bytes());
        table(b"SPCR", revision, &body)
    }

    #[test]
    fn spcr_reads_pl011_in_memory() {
        assert_eq!(
            Spcr::parse(&spcr(2, 3, 0, 0x0900_0000)),
            Ok(Spcr {
                interface: SerialInterface::Pl011,
                base: Some(GenericAddress {
                    space: AddressSpace::Memory,
                    raw_value: 0x0900_0000,
                }),
            })
        );
    }

    #[test]
    fn spcr_reads_16550_in_io_space() {
        assert_eq!(
            Spcr::parse(&spcr(1, 0, 1, 0x3f8)),
            Ok(Spcr {
                interface: SerialInterface::Uart16550,
                base: Some(GenericAddress {
                    space: AddressSpace::Io,
                    raw_value: 0x3f8,
                }),
            })
        );
    }

    #[test]
    fn spcr_keeps_code_three_reserved_in_revision_one() {
        assert_eq!(
            Spcr::parse(&spcr(1, 3, 0, 0x0900_0000)).map(|spcr| spcr.interface),
            Ok(SerialInterface::Other(3))
        );
    }

    #[test]
    fn spcr_reports_an_unknown_interface() {
        assert_eq!(
            Spcr::parse(&spcr(4, 0x0e, 0, 0x0900_0000)).map(|spcr| spcr.interface),
            Ok(SerialInterface::Other(0x0e))
        );
    }

    #[test]
    fn spcr_without_a_base_means_redirection_is_disabled() {
        assert_eq!(
            Spcr::parse(&spcr(2, 3, 0, 0)).map(|spcr| spcr.base),
            Ok(None)
        );
    }

    #[test]
    fn spcr_rejects_a_short_table() {
        let bytes = table(b"SPCR", 2, &[0; 15]);
        assert_eq!(Spcr::parse(&bytes), Err(AcpiError::Length));
    }
}
