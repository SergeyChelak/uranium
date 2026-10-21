use address::{AddressLayout, VirtualAddress};

pub enum Layout {}

impl AddressLayout for Layout {
    const VIRTUAL_ADDRESS_BIT_COUNT: u32 = 48;
    const PHYSICAL_ADDRESS_BIT_COUNT: u32 = 48;
    const KERNEL_HALF_START: VirtualAddress = VirtualAddress::new(0xffff_0000_0000_0000);
    const PHYSICAL_MAP_BASE: VirtualAddress = VirtualAddress::new(0xffff_8000_0000_0000);
    const PHYSICAL_MAP_SIZE: u64 = 64 << 40;

    fn is_canonical(address: VirtualAddress) -> bool {
        let region_selector = address.as_u64() >> Self::VIRTUAL_ADDRESS_BIT_COUNT;
        region_selector == 0 || region_selector == u64::MAX >> Self::VIRTUAL_ADDRESS_BIT_COUNT
    }
}

#[cfg(test)]
mod tests {
    use address::{AddressLayout, VirtualAddress};

    use super::Layout;

    #[test]
    fn low_range_is_canonical() {
        assert!(VirtualAddress::canonical::<Layout>(0x0000_ffff_ffff_ffff).is_some());
    }

    #[test]
    fn high_range_is_canonical() {
        assert!(VirtualAddress::canonical::<Layout>(0xffff_0000_0000_0000).is_some());
    }

    #[test]
    fn hole_is_not_canonical() {
        assert!(VirtualAddress::canonical::<Layout>(0x0001_0000_0000_0000).is_none());
        assert!(VirtualAddress::canonical::<Layout>(0xfffe_ffff_ffff_ffff).is_none());
    }

    #[test]
    fn windows_match_the_design() {
        assert_eq!(Layout::KERNEL_HALF_START.as_u64(), 0xffff_0000_0000_0000);
        assert_eq!(Layout::PHYSICAL_MAP_BASE.as_u64(), 0xffff_8000_0000_0000);
        assert_eq!(Layout::PHYSICAL_MAP_SIZE, 0x4000_0000_0000);
    }

    #[test]
    fn both_address_kinds_have_forty_eight_bits() {
        assert_eq!(Layout::VIRTUAL_ADDRESS_BIT_COUNT, 48);
        assert_eq!(Layout::PHYSICAL_ADDRESS_BIT_COUNT, 48);
    }
}
