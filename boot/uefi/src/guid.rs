// UEFI 2.11, 7.3.2 (structure) and Appendix A (byte order)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

impl Guid {
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Guid;
    use core::mem::{offset_of, size_of};

    #[test]
    fn guid_is_sixteen_bytes() {
        assert_eq!(size_of::<Guid>(), 16);
    }

    #[test]
    fn fields_lie_at_the_specified_offsets() {
        assert_eq!(offset_of!(Guid, data1), 0);
        assert_eq!(offset_of!(Guid, data2), 4);
        assert_eq!(offset_of!(Guid, data3), 6);
        assert_eq!(offset_of!(Guid, data4), 8);
    }

    #[test]
    fn equal_fields_give_equal_guids() {
        let serial_io = Guid::new(
            0xBB25_CF6F,
            0xF1D4,
            0x11D2,
            [0x9a, 0x0c, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0xfd],
        );
        assert_eq!(serial_io, serial_io);
        assert_ne!(serial_io, Guid::new(0, 0, 0, [0; 8]));
    }
}
