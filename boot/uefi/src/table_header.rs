// UEFI 2.11, 4.2.1
#[repr(C)]
pub(crate) struct TableHeader {
    signature: u64,
    revision: u32,
    header_size: u32,
    crc32: u32,
    reserved: u32,
}

impl TableHeader {
    /// Whether the table that follows is of the kind `expected` names
    pub(crate) fn has_signature(&self, expected: u64) -> bool {
        self.signature == expected
    }
}

#[cfg(test)]
impl TableHeader {
    /// The header of a fake table; every other field stays zero
    pub(crate) const fn with_signature(signature: u64) -> Self {
        Self {
            signature,
            revision: 0,
            header_size: 0,
            crc32: 0,
            reserved: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TableHeader;
    use core::mem::{offset_of, size_of};

    #[test]
    fn has_signature_compares_the_signature_field() {
        let header = TableHeader::with_signature(0x1234);
        assert!(header.has_signature(0x1234));
        assert!(!header.has_signature(0x4321));
    }

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
