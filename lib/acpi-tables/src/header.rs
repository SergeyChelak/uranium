use byte_fields::Fields;

use crate::AcpiError;

// ACPI 6.6, 5.2.6
const SIGNATURE_OFFSET: usize = 0;
const LENGTH_OFFSET: usize = 4;
const REVISION_OFFSET: usize = 8;

/// The header every system description table starts with
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
}

impl TableHeader {
    // ACPI 6.6, 5.2.6: the last header field ends at byte 36
    pub const LENGTH: usize = 36;

    pub fn parse(bytes: &[u8]) -> Result<Self, AcpiError> {
        let fields = Fields::exactly(bytes, Self::LENGTH)?;
        let length = fields.u32(LENGTH_OFFSET)?;
        if usize::try_from(length).map_or(true, |length| length < Self::LENGTH) {
            return Err(AcpiError::Length);
        }
        Ok(Self {
            signature: fields.array(SIGNATURE_OFFSET)?,
            length,
            revision: fields.u8(REVISION_OFFSET)?,
        })
    }

    /// The table behind a header that carries `signature`, cut to its length,
    /// once every byte of it has been proven to sum to zero
    pub(crate) fn verify<'a>(
        bytes: &'a [u8],
        signature: &[u8; 4],
    ) -> Result<(Self, Fields<'a>), AcpiError> {
        let header = Self::parse(bytes)?;
        if header.signature != *signature {
            return Err(AcpiError::Signature);
        }
        let length = usize::try_from(header.length).map_err(|_| AcpiError::Length)?;
        let table = Fields::exactly(bytes, length)?;
        if !sums_to_zero(table.bytes()) {
            return Err(AcpiError::Checksum);
        }
        Ok((header, table))
    }
}

/// Every byte of a table, its checksum included, adds up to zero
pub(crate) fn sums_to_zero(bytes: &[u8]) -> bool {
    bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) == 0
}

#[cfg(test)]
pub(crate) mod testing {
    use std::vec;
    use std::vec::Vec;

    use crate::TableHeader;

    /// Sets the byte at `checksum_offset` so that all of `bytes` sum to zero
    pub(crate) fn with_checksum(mut bytes: Vec<u8>, checksum_offset: usize) -> Vec<u8> {
        bytes[checksum_offset] = 0;
        let sum = bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        bytes[checksum_offset] = sum.wrapping_neg();
        bytes
    }

    /// A table with a valid header: the signature, the length over everything,
    /// the revision and the checksum; the vendor fields stay zero
    pub(crate) fn table(signature: &[u8; 4], revision: u8, body: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0u8; TableHeader::LENGTH];
        bytes[..4].copy_from_slice(signature);
        let length = u32::try_from(TableHeader::LENGTH + body.len()).unwrap();
        bytes[4..8].copy_from_slice(&length.to_le_bytes());
        bytes[8] = revision;
        bytes.extend_from_slice(body);
        with_checksum(bytes, 9)
    }
}

#[cfg(test)]
mod tests {
    use super::testing::table;
    use super::{TableHeader, sums_to_zero};
    use crate::AcpiError;

    #[test]
    fn sums_to_zero_wraps_around_at_256() {
        assert!(sums_to_zero(&[0x80, 0x80]));
        assert!(sums_to_zero(&[]));
        assert!(!sums_to_zero(&[0x80, 0x7f]));
    }

    #[test]
    fn header_parses_signature_length_and_revision() {
        let bytes = table(b"APIC", 5, &[1, 2, 3]);
        assert_eq!(
            TableHeader::parse(&bytes),
            Ok(TableHeader {
                signature: *b"APIC",
                length: 39,
                revision: 5,
            })
        );
    }

    #[test]
    fn header_rejects_a_short_buffer() {
        let bytes = table(b"APIC", 1, &[]);
        assert_eq!(
            TableHeader::parse(&bytes[..TableHeader::LENGTH - 1]),
            Err(AcpiError::Length)
        );
    }

    #[test]
    fn header_rejects_a_length_below_the_header() {
        let mut bytes = table(b"APIC", 1, &[]);
        bytes[4..8].copy_from_slice(&35u32.to_le_bytes());
        assert_eq!(TableHeader::parse(&bytes), Err(AcpiError::Length));
    }

    #[test]
    fn verify_hands_out_the_table_cut_to_its_length() {
        let mut bytes = table(b"APIC", 1, &[7, 7]);
        bytes.push(0xee);
        let (header, verified) = TableHeader::verify(&bytes, b"APIC").unwrap();
        assert_eq!(header.length, 38);
        assert_eq!(verified.tail(0), Ok(&bytes[..38]));
    }

    #[test]
    fn verify_rejects_a_wrong_signature() {
        let bytes = table(b"APIC", 1, &[]);
        assert_eq!(
            TableHeader::verify(&bytes, b"XSDT").map(|(header, _)| header),
            Err(AcpiError::Signature)
        );
    }

    #[test]
    fn verify_rejects_a_buffer_shorter_than_the_length() {
        let bytes = table(b"APIC", 1, &[7, 7]);
        assert_eq!(
            TableHeader::verify(&bytes[..37], b"APIC").map(|(header, _)| header),
            Err(AcpiError::Length)
        );
    }

    #[test]
    fn verify_rejects_a_bad_checksum() {
        let mut bytes = table(b"APIC", 1, &[7, 7]);
        bytes[36] ^= 1;
        assert_eq!(
            TableHeader::verify(&bytes, b"APIC").map(|(header, _)| header),
            Err(AcpiError::Checksum)
        );
    }
}
