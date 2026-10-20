use crate::{Decoder, OutOfBounds};

/// The bytes of one structure, read field by field at known offsets
#[derive(Clone, Copy)]
pub struct Fields<'a>(&'a [u8]);

impl<'a> Fields<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    /// The first `length` bytes, or `OutOfBounds` when there are fewer
    pub fn exactly(bytes: &'a [u8], length: usize) -> Result<Self, OutOfBounds> {
        bytes.get(..length).map(Self).ok_or(OutOfBounds)
    }

    pub fn u8(self, offset: usize) -> Result<u8, OutOfBounds> {
        self.0.get(offset).copied().ok_or(OutOfBounds)
    }

    pub fn u16(self, offset: usize) -> Result<u16, OutOfBounds> {
        self.array(offset).map(u16::from_le_bytes)
    }

    pub fn u32(self, offset: usize) -> Result<u32, OutOfBounds> {
        self.array(offset).map(u32::from_le_bytes)
    }

    pub fn u64(self, offset: usize) -> Result<u64, OutOfBounds> {
        self.array(offset).map(u64::from_le_bytes)
    }

    pub fn array<const N: usize>(self, offset: usize) -> Result<[u8; N], OutOfBounds> {
        self.tail(offset)?
            .first_chunk::<N>()
            .copied()
            .ok_or(OutOfBounds)
    }

    /// Everything from `offset` to the end
    pub fn tail(self, offset: usize) -> Result<&'a [u8], OutOfBounds> {
        self.0.get(offset..).ok_or(OutOfBounds)
    }

    pub fn starts_with(self, prefix: &[u8]) -> bool {
        self.0.starts_with(prefix)
    }

    pub fn bytes(self) -> &'a [u8] {
        self.0
    }

    /// A cursor that starts reading at `offset`
    pub fn at(self, offset: usize) -> Decoder<'a> {
        Decoder::over(self, offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BYTES: [u8; 13] = [0xff, 0x78, 0x56, 0x34, 0x12, 1, 2, 3, 4, 5, 6, 7, 8];

    #[test]
    fn exactly_cuts_to_the_length() {
        let fields = Fields::exactly(&BYTES, 2).unwrap();
        assert_eq!(fields.tail(0), Ok(&BYTES[..2]));
        assert_eq!(fields.u8(2), Err(OutOfBounds));
    }

    #[test]
    fn exactly_rejects_a_short_buffer() {
        assert_eq!(Fields::exactly(&BYTES, 14).err(), Some(OutOfBounds));
    }

    #[test]
    fn numbers_are_little_endian() {
        let fields = Fields::new(&BYTES);
        assert_eq!(fields.u16(1), Ok(0x5678));
        assert_eq!(fields.u32(1), Ok(0x1234_5678));
    }

    #[test]
    fn u64_refuses_a_field_past_the_end() {
        let fields = Fields::new(&BYTES);
        assert_eq!(fields.u64(5), Ok(0x0807_0605_0403_0201));
        assert_eq!(fields.u64(6), Err(OutOfBounds));
    }

    #[test]
    fn array_reads_at_an_offset() {
        let fields = Fields::new(&BYTES);
        assert_eq!(fields.array(11), Ok([7, 8]));
        assert_eq!(fields.array::<3>(11), Err(OutOfBounds));
    }

    #[test]
    fn starts_with_looks_at_the_first_bytes() {
        let fields = Fields::new(b"RSD PTR tail");
        assert!(fields.starts_with(b"RSD PTR "));
        assert!(!fields.starts_with(b"XSDT"));
    }

    #[test]
    fn at_starts_a_decoder_at_the_offset() {
        let mut decoder = Fields::new(&BYTES).at(5);
        assert_eq!(decoder.read::<u8>(), Ok(1));
        assert_eq!(decoder.position(), 6);
    }
}
