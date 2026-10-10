crate::address_type::address_type!(PhysicalAddress);

#[cfg(test)]
mod tests {
    use std::format;

    use super::PhysicalAddress;

    #[test]
    fn new_and_as_u64_round_trip() {
        assert_eq!(PhysicalAddress::new(0x1234).as_u64(), 0x1234);
    }

    #[test]
    fn checked_add_returns_none_on_overflow() {
        assert_eq!(
            PhysicalAddress::new(0x1000).checked_add(0x10),
            Some(PhysicalAddress::new(0x1010))
        );
        assert_eq!(PhysicalAddress::new(u64::MAX).checked_add(1), None);
    }

    #[test]
    fn add_moves_by_offset() {
        assert_eq!(
            PhysicalAddress::new(0x1000) + 0x234,
            PhysicalAddress::new(0x1234)
        );
    }

    #[test]
    #[should_panic(expected = "overflows the address space")]
    fn add_panics_on_overflow() {
        let _ = PhysicalAddress::new(u64::MAX) + 1;
    }

    #[test]
    fn sub_gives_distance() {
        assert_eq!(
            PhysicalAddress::new(0x1234) - PhysicalAddress::new(0x1000),
            0x234
        );
    }

    #[test]
    #[should_panic(expected = "above the minuend")]
    fn sub_panics_when_reversed() {
        let _ = PhysicalAddress::new(0x1000) - PhysicalAddress::new(0x1234);
    }

    #[test]
    fn align_down_and_up() {
        let address = PhysicalAddress::new(0x1234);
        assert_eq!(address.align_down(0x1000), PhysicalAddress::new(0x1000));
        assert_eq!(address.align_up(0x1000), PhysicalAddress::new(0x2000));
    }

    #[test]
    fn align_up_keeps_aligned_value() {
        let address = PhysicalAddress::new(0x2000);
        assert_eq!(address.align_up(0x1000), address);
    }

    #[test]
    fn is_aligned() {
        assert!(PhysicalAddress::new(0x2000).is_aligned(0x1000));
        assert!(!PhysicalAddress::new(0x2008).is_aligned(0x1000));
    }

    #[test]
    fn debug_prints_hexadecimal() {
        assert_eq!(
            format!("{:?}", PhysicalAddress::new(0x1000)),
            "PhysicalAddress(0x0000000000001000)"
        );
        assert_eq!(format!("{:#x}", PhysicalAddress::new(0x1000)), "0x1000");
    }

    #[test]
    fn orders_by_value() {
        assert!(PhysicalAddress::new(0x1000) < PhysicalAddress::new(0x2000));
    }
}
