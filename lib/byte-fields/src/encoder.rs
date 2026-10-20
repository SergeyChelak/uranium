use crate::OutOfBounds;

/// A type that is written as a run of consecutive fields
pub trait Encode {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), OutOfBounds>;
}

/// Writes fields one after another; a field that does not fit leaves the
/// position and the bytes where they were
pub struct Encoder<'a> {
    bytes: &'a mut [u8],
    position: usize,
}

impl<'a> Encoder<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub fn write<T: Encode + ?Sized>(&mut self, value: &T) -> Result<(), OutOfBounds> {
        value.encode(self)
    }

    pub fn bytes(&mut self, value: &[u8]) -> Result<(), OutOfBounds> {
        let end = self.position.checked_add(value.len()).ok_or(OutOfBounds)?;
        self.bytes
            .get_mut(self.position..end)
            .ok_or(OutOfBounds)?
            .copy_from_slice(value);
        self.position = end;
        Ok(())
    }

    pub fn position(&self) -> usize {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_lays_fields_in_order() {
        let mut block = [0u8; 8];
        let mut encoder = Encoder::new(&mut block);
        assert_eq!(encoder.write(&1u8), Ok(()));
        assert_eq!(encoder.write(&0x1234u16), Ok(()));
        assert_eq!(encoder.write(&0x1234_5678u32), Ok(()));
        assert_eq!(encoder.bytes(&[7]), Ok(()));
        assert_eq!(encoder.position(), 8);
        assert_eq!(block, [1, 0x34, 0x12, 0x78, 0x56, 0x34, 0x12, 7]);
    }

    #[test]
    fn a_short_write_does_not_move_the_encoder() {
        let mut block = [0u8; 3];
        let mut encoder = Encoder::new(&mut block);
        assert_eq!(encoder.write(&0x0201u16), Ok(()));
        assert_eq!(encoder.write(&0u32), Err(OutOfBounds));
        assert_eq!(encoder.position(), 2);
        assert_eq!(encoder.write(&3u8), Ok(()));
        assert_eq!(block, [1, 2, 3]);
    }
}
