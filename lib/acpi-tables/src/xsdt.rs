//! The Extended System Description Table (XSDT): the list of 64-bit
//! addresses of every other ACPI table

use address::PhysicalAddress;

use crate::{AcpiError, TableHeader};

// ACPI 6.6, 5.2.8
const SIGNATURE: &[u8; 4] = b"XSDT";
const ENTRY_LENGTH: usize = 8;

pub struct Xsdt;

impl Xsdt {
    /// The addresses of the other tables, offered only after the whole table
    /// has been verified
    pub fn entries(bytes: &[u8]) -> Result<impl Iterator<Item = PhysicalAddress> + '_, AcpiError> {
        let (_, table) = TableHeader::verify(bytes, SIGNATURE)?;
        let (entries, ragged_tail) = table.tail(TableHeader::LENGTH)?.as_chunks::<ENTRY_LENGTH>();
        if !ragged_tail.is_empty() {
            return Err(AcpiError::Length);
        }
        Ok(entries
            .iter()
            .map(|entry| PhysicalAddress::new(u64::from_le_bytes(*entry))))
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use address::PhysicalAddress;

    use super::Xsdt;
    use crate::AcpiError;
    use crate::fields::testing::table;

    fn addresses(bytes: &[u8]) -> Result<Vec<PhysicalAddress>, AcpiError> {
        Xsdt::entries(bytes).map(Iterator::collect)
    }

    #[test]
    fn xsdt_lists_its_entries() {
        let mut body = Vec::new();
        body.extend_from_slice(&0x7fe0_0000u64.to_le_bytes());
        body.extend_from_slice(&0x1_0000_0000u64.to_le_bytes());
        let bytes = table(b"XSDT", 1, &body);
        assert_eq!(
            addresses(&bytes),
            Ok([0x7fe0_0000, 0x1_0000_0000]
                .map(PhysicalAddress::new)
                .to_vec())
        );
    }

    #[test]
    fn xsdt_without_entries_is_empty() {
        assert_eq!(addresses(&table(b"XSDT", 1, &[])), Ok(Vec::new()));
    }

    #[test]
    fn xsdt_rejects_a_ragged_entry_array() {
        let bytes = table(b"XSDT", 1, &[0; 12]);
        assert_eq!(addresses(&bytes), Err(AcpiError::Length));
    }

    #[test]
    fn xsdt_rejects_another_table() {
        let bytes = table(b"RSDT", 1, &[0; 8]);
        assert_eq!(addresses(&bytes), Err(AcpiError::Signature));
    }

    #[test]
    fn xsdt_rejects_a_bad_checksum() {
        let mut bytes = table(b"XSDT", 1, &[0; 8]);
        bytes[40] = 1;
        assert_eq!(addresses(&bytes), Err(AcpiError::Checksum));
    }
}
