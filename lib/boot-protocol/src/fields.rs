//! Little-endian fields at byte offsets, the one way this crate reads and
//! writes the wire format without touching a structure's memory directly

pub(crate) fn u64_at(bytes: &[u8], offset: usize) -> Option<u64> {
    let field = bytes.get(offset..offset.checked_add(8)?)?;
    Some(u64::from_le_bytes(field.try_into().ok()?))
}

pub(crate) fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let field = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes(field.try_into().ok()?))
}

pub(crate) fn put_u64(block: &mut [u8], offset: usize, value: u64) {
    field_mut(block, offset, 8).copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_u32(block: &mut [u8], offset: usize, value: u32) {
    field_mut(block, offset, 4).copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_bytes(block: &mut [u8], offset: usize, value: &[u8]) {
    field_mut(block, offset, value.len()).copy_from_slice(value);
}

fn field_mut(block: &mut [u8], offset: usize, length: usize) -> &mut [u8] {
    offset
        .checked_add(length)
        .and_then(|end| block.get_mut(offset..end))
        .expect("the block was measured before it was written")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_are_little_endian() {
        let mut block = [0u8; 16];
        put_u64(&mut block, 0, 0x0102_0304_0506_0708);
        put_u32(&mut block, 8, 0x0a0b_0c0d);
        put_bytes(&mut block, 12, b"wire");
        assert_eq!(block[..8], [8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(block[8..12], [0x0d, 0x0c, 0x0b, 0x0a]);
        assert_eq!(u64_at(&block, 0), Some(0x0102_0304_0506_0708));
        assert_eq!(u32_at(&block, 8), Some(0x0a0b_0c0d));
        assert_eq!(&block[12..], b"wire");
    }

    #[test]
    fn a_field_past_the_end_is_none() {
        let block = [0u8; 10];
        assert_eq!(u64_at(&block, 4), None);
        assert_eq!(u32_at(&block, 8), None);
        assert_eq!(u64_at(&block, usize::MAX), None);
    }

    #[test]
    #[should_panic(expected = "measured")]
    fn writing_past_the_end_is_a_broken_invariant() {
        let mut block = [0u8; 10];
        put_u64(&mut block, 4, 0);
    }
}
