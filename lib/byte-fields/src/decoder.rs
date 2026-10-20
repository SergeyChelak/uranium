use crate::{Fields, OutOfBounds};

/// A type that is read from a run of consecutive fields
pub trait Decode: Sized {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, OutOfBounds>;
}

/// Reads fields one after another; a field that does not fit leaves the
/// position where it was
pub struct Decoder<'a> {
    fields: Fields<'a>,
    position: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self::over(Fields::new(bytes), 0)
    }

    pub(crate) fn over(fields: Fields<'a>, position: usize) -> Self {
        Self { fields, position }
    }

    pub fn read<T: Decode>(&mut self) -> Result<T, OutOfBounds> {
        T::decode(self)
    }

    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], OutOfBounds> {
        let array = self.fields.array(self.position)?;
        self.position += N;
        Ok(array)
    }

    /// The next `length` bytes as they lie
    pub fn bytes(&mut self, length: usize) -> Result<&'a [u8], OutOfBounds> {
        let end = self.position.checked_add(length).ok_or(OutOfBounds)?;
        let bytes = self
            .fields
            .bytes()
            .get(self.position..end)
            .ok_or(OutOfBounds)?;
        self.position = end;
        Ok(bytes)
    }

    pub fn skip(&mut self, length: usize) -> Result<(), OutOfBounds> {
        self.bytes(length).map(|_| ())
    }

    pub fn position(&self) -> usize {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_walks_fields_in_order() {
        let bytes = [1, 0x34, 0x12, 0x78, 0x56, 0x34, 0x12, 9, 9, 9, 7];
        let mut decoder = Decoder::new(&bytes);
        assert_eq!(decoder.read::<u8>(), Ok(1));
        assert_eq!(decoder.read::<u16>(), Ok(0x1234));
        assert_eq!(decoder.read::<u32>(), Ok(0x1234_5678));
        assert_eq!(decoder.skip(3), Ok(()));
        assert_eq!(decoder.bytes(1), Ok(&[7][..]));
        assert_eq!(decoder.position(), 11);
    }

    #[test]
    fn a_short_read_does_not_move_the_decoder() {
        let bytes = [1, 2, 3];
        let mut decoder = Decoder::new(&bytes);
        assert_eq!(decoder.read::<u16>(), Ok(0x0201));
        assert_eq!(decoder.read::<u32>(), Err(OutOfBounds));
        assert_eq!(decoder.skip(2), Err(OutOfBounds));
        assert_eq!(decoder.position(), 2);
        assert_eq!(decoder.read::<u8>(), Ok(3));
    }
}
