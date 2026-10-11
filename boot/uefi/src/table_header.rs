// UEFI 2.11, 4.2.1
#[repr(C)]
pub struct TableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[cfg(test)]
mod tests {
    use super::TableHeader;
    use core::mem::{offset_of, size_of};

    #[test]
    fn table_header_is_24_bytes() {
        assert_eq!(size_of::<TableHeader>(), 24);
    }

    #[test]
    fn fields_lie_at_the_specified_offsets() {
        assert_eq!(offset_of!(TableHeader, signature), 0);
        assert_eq!(offset_of!(TableHeader, revision), 8);
        assert_eq!(offset_of!(TableHeader, header_size), 12);
        assert_eq!(offset_of!(TableHeader, crc32), 16);
        assert_eq!(offset_of!(TableHeader, reserved), 20);
    }
}
