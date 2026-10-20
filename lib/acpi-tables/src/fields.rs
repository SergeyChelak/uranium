// ACPI 6.6, 5.2.1: every number in a table is little-endian

use crate::AcpiError;

/// The bytes of one structure, read field by field at known offsets; a field
/// that reaches past the end is `AcpiError::Length`
#[derive(Clone, Copy)]
pub(crate) struct Fields<'a>(&'a [u8]);

impl<'a> Fields<'a> {
    /// The first `length` bytes, or `AcpiError::Length` when there are fewer
    pub(crate) fn exactly(bytes: &'a [u8], length: usize) -> Result<Self, AcpiError> {
        bytes.get(..length).map(Self).ok_or(AcpiError::Length)
    }

    pub(crate) fn u8(&self, offset: usize) -> Result<u8, AcpiError> {
        self.0.get(offset).copied().ok_or(AcpiError::Length)
    }

    pub(crate) fn u32(&self, offset: usize) -> Result<u32, AcpiError> {
        self.array(offset).map(u32::from_le_bytes)
    }

    pub(crate) fn u64(&self, offset: usize) -> Result<u64, AcpiError> {
        self.array(offset).map(u64::from_le_bytes)
    }

    pub(crate) fn array<const N: usize>(&self, offset: usize) -> Result<[u8; N], AcpiError> {
        self.tail(offset)?
            .first_chunk::<N>()
            .copied()
            .ok_or(AcpiError::Length)
    }

    /// Everything from `offset` to the end
    pub(crate) fn tail(&self, offset: usize) -> Result<&'a [u8], AcpiError> {
        self.0.get(offset..).ok_or(AcpiError::Length)
    }

    pub(crate) fn starts_with(&self, prefix: &[u8]) -> bool {
        self.0.starts_with(prefix)
    }

    pub(crate) fn sums_to_zero(&self) -> bool {
        self.0.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) == 0
    }
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
    use super::Fields;
    use crate::AcpiError;

    const BYTES: [u8; 13] = [0xff, 0x78, 0x56, 0x34, 0x12, 1, 2, 3, 4, 5, 6, 7, 8];

    #[test]
    fn exactly_cuts_to_the_length() {
        let fields = Fields::exactly(&BYTES, 2).unwrap();
        assert_eq!(fields.tail(0), Ok(&BYTES[..2]));
        assert_eq!(fields.u8(2), Err(AcpiError::Length));
    }

    #[test]
    fn exactly_rejects_a_short_buffer() {
        assert!(Fields::exactly(&BYTES, 14).is_err());
    }

    #[test]
    fn u32_is_little_endian() {
        let fields = Fields::exactly(&BYTES, BYTES.len()).unwrap();
        assert_eq!(fields.u32(1), Ok(0x1234_5678));
    }

    #[test]
    fn u64_refuses_a_field_past_the_end() {
        let fields = Fields::exactly(&BYTES, BYTES.len()).unwrap();
        assert_eq!(fields.u64(5), Ok(0x0807_0605_0403_0201));
        assert_eq!(fields.u64(6), Err(AcpiError::Length));
    }

    #[test]
    fn array_reads_at_an_offset() {
        let fields = Fields::exactly(&BYTES, BYTES.len()).unwrap();
        assert_eq!(fields.array(11), Ok([7, 8]));
        assert_eq!(fields.array::<3>(11), Err(AcpiError::Length));
    }

    #[test]
    fn sums_to_zero_wraps_around_at_256() {
        assert!(Fields::exactly(&[0x80, 0x80], 2).unwrap().sums_to_zero());
        assert!(Fields::exactly(&[], 0).unwrap().sums_to_zero());
        assert!(!Fields::exactly(&[0x80, 0x7f], 2).unwrap().sums_to_zero());
    }
}
