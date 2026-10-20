//! The Root System Description Pointer (RSDP): the structure the firmware
//! leaves in its configuration tables to say where the other ACPI tables are

use address::PhysicalAddress;

use crate::AcpiError;
use crate::fields::Fields;

// ACPI 6.6, 5.2.5.3
const SIGNATURE: &[u8; 8] = b"RSD PTR ";
const FIRST_CHECKSUM_LENGTH: usize = 20;
const REVISION_OFFSET: usize = 15;
const LENGTH_OFFSET: usize = 20;
const XSDT_ADDRESS_OFFSET: usize = 24;
// ACPI 6.6, 5.2.5.3: the XSDT address exists from revision 2
const XSDT_REVISION: u8 = 2;

/// The root pointer, reduced to the one address the loader follows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rsdp {
    pub xsdt: PhysicalAddress,
}

impl Rsdp {
    // ACPI 6.6, 5.2.5.3: the reserved bytes of revision 2 end at byte 36
    pub const LENGTH: usize = 36;

    pub fn parse(bytes: &[u8]) -> Result<Self, AcpiError> {
        let fields = Fields::exactly(bytes, Self::LENGTH)?;
        if !fields.starts_with(SIGNATURE) {
            return Err(AcpiError::Signature);
        }
        if !Fields::exactly(bytes, FIRST_CHECKSUM_LENGTH)?.sums_to_zero() {
            return Err(AcpiError::Checksum);
        }
        if fields.u8(REVISION_OFFSET)? < XSDT_REVISION {
            return Err(AcpiError::Revision);
        }
        let length = fields.u32(LENGTH_OFFSET)?;
        // A longer structure would keep bytes of its extended checksum outside
        // this copy, and no revision above 2 defines one
        if usize::try_from(length) != Ok(Self::LENGTH) {
            return Err(AcpiError::Length);
        }
        if !fields.sums_to_zero() {
            return Err(AcpiError::Checksum);
        }
        Ok(Self {
            xsdt: PhysicalAddress::new(fields.u64(XSDT_ADDRESS_OFFSET)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use address::PhysicalAddress;

    use super::Rsdp;
    use crate::AcpiError;
    use crate::fields::testing::with_checksum;

    fn rsdp(revision: u8, length: u32, xsdt: u64) -> Vec<u8> {
        let mut bytes = [0u8; Rsdp::LENGTH];
        bytes[..8].copy_from_slice(b"RSD PTR ");
        bytes[15] = revision;
        bytes[20..24].copy_from_slice(&length.to_le_bytes());
        bytes[24..32].copy_from_slice(&xsdt.to_le_bytes());
        let mut first = with_checksum(bytes[..20].to_vec(), 8);
        first.extend_from_slice(&bytes[20..]);
        with_checksum(first, 32)
    }

    #[test]
    fn rsdp_points_at_the_xsdt() {
        let bytes = rsdp(2, 36, 0x7fff_0000);
        assert_eq!(
            Rsdp::parse(&bytes),
            Ok(Rsdp {
                xsdt: PhysicalAddress::new(0x7fff_0000),
            })
        );
    }

    #[test]
    fn rsdp_rejects_a_bad_signature() {
        let mut bytes = rsdp(2, 36, 0x7fff_0000);
        bytes[7] = b'!';
        assert_eq!(Rsdp::parse(&bytes), Err(AcpiError::Signature));
    }

    #[test]
    fn rsdp_rejects_a_bad_checksum() {
        let mut bytes = rsdp(2, 36, 0x7fff_0000);
        bytes[16] ^= 1;
        assert_eq!(Rsdp::parse(&bytes), Err(AcpiError::Checksum));
    }

    #[test]
    fn rsdp_requires_revision_two() {
        let bytes = rsdp(0, 36, 0x7fff_0000);
        assert_eq!(Rsdp::parse(&bytes), Err(AcpiError::Revision));
    }

    #[test]
    fn rsdp_rejects_an_unexpected_length() {
        assert_eq!(Rsdp::parse(&rsdp(2, 20, 0)), Err(AcpiError::Length));
        assert_eq!(Rsdp::parse(&rsdp(2, 40, 0)), Err(AcpiError::Length));
        assert_eq!(Rsdp::parse(&rsdp(2, 36, 0)[..35]), Err(AcpiError::Length));
    }

    #[test]
    fn rsdp_rejects_a_bad_extended_checksum() {
        let mut bytes = rsdp(2, 36, 0x7fff_0000);
        bytes[33] = 1;
        assert_eq!(Rsdp::parse(&bytes), Err(AcpiError::Checksum));
    }
}
