use address::{AddressLayout, VirtualAddress};

pub enum Layout {}

impl AddressLayout for Layout {
    const VIRTUAL_ADDRESS_BIT_COUNT: u32 = 48;
    const PHYSICAL_ADDRESS_BIT_COUNT: u32 = 52;
    const KERNEL_HALF_START: VirtualAddress = VirtualAddress::new(0xffff_8000_0000_0000);
    const PHYSICAL_MAP_BASE: VirtualAddress = VirtualAddress::new(0xffff_8000_0000_0000);
}

#[cfg(test)]
mod tests {
    use address::{AddressLayout, VirtualAddress};

    use super::Layout;

    #[test]
    fn low_half_is_canonical() {
        assert!(VirtualAddress::canonical::<Layout>(0x0000_7fff_ffff_ffff).is_some());
    }

    #[test]
    fn high_half_is_canonical() {
        assert!(VirtualAddress::canonical::<Layout>(0xffff_8000_0000_0000).is_some());
    }

    #[test]
    fn hole_is_not_canonical() {
        assert!(VirtualAddress::canonical::<Layout>(0x0000_8000_0000_0000).is_none());
        assert!(VirtualAddress::canonical::<Layout>(0xffff_7fff_ffff_ffff).is_none());
    }

    #[test]
    fn windows_match_the_design() {
        assert_eq!(Layout::KERNEL_HALF_START.as_u64(), 0xffff_8000_0000_0000);
        assert_eq!(Layout::PHYSICAL_MAP_BASE.as_u64(), 0xffff_8000_0000_0000);
    }

    #[test]
    fn physical_addresses_have_fifty_two_bits() {
        assert_eq!(Layout::PHYSICAL_ADDRESS_BIT_COUNT, 52);
        assert_eq!(Layout::VIRTUAL_ADDRESS_BIT_COUNT, 48);
    }
}
