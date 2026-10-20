use crate::AcpiError;
use crate::fields::Fields;

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
        if !table.sums_to_zero() {
            return Err(AcpiError::Checksum);
        }
        Ok((header, table))
    }
}

#[cfg(test)]
mod tests {
    use super::TableHeader;
    use crate::AcpiError;
    use crate::fields::testing::table;

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
