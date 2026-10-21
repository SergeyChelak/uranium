use crate::VirtualAddress;

pub trait AddressLayout {
    const VIRTUAL_ADDRESS_BIT_COUNT: u32;
    const PHYSICAL_ADDRESS_BIT_COUNT: u32;
    const KERNEL_HALF_START: VirtualAddress;
    const PHYSICAL_MAP_BASE: VirtualAddress;
    /// How much of physical memory the window at `PHYSICAL_MAP_BASE` shows,
    /// counted from physical address zero
    const PHYSICAL_MAP_SIZE: u64;

    fn is_canonical(address: VirtualAddress) -> bool {
        is_sign_extended(address.as_u64(), Self::VIRTUAL_ADDRESS_BIT_COUNT)
    }
}

fn is_sign_extended(raw: u64, bits: u32) -> bool {
    let shift = bits - 1;
    let upper_bits = raw >> shift;
    upper_bits == 0 || upper_bits == u64::MAX >> shift
}

pub(crate) fn fits_in_bits(raw: u64, bits: u32) -> bool {
    bits >= u64::BITS || raw >> bits == 0
}

#[cfg(test)]
mod tests {
    use super::AddressLayout;
    use crate::{PhysicalAddress, VirtualAddress};

    enum TestLayout {}

    impl AddressLayout for TestLayout {
        const VIRTUAL_ADDRESS_BIT_COUNT: u32 = 48;
        const PHYSICAL_ADDRESS_BIT_COUNT: u32 = 52;
        const KERNEL_HALF_START: VirtualAddress = VirtualAddress::new(0xffff_8000_0000_0000);
        const PHYSICAL_MAP_BASE: VirtualAddress = VirtualAddress::new(0xffff_8000_0000_0000);
        const PHYSICAL_MAP_SIZE: u64 = 64 << 40;
    }

    #[test]
    fn canonical_accepts_low_half() {
        assert_eq!(
            VirtualAddress::canonical::<TestLayout>(0x0000_7fff_ffff_ffff),
            Some(VirtualAddress::new(0x0000_7fff_ffff_ffff))
        );
        assert!(VirtualAddress::canonical::<TestLayout>(0).is_some());
    }

    #[test]
    fn canonical_accepts_high_half() {
        assert_eq!(
            VirtualAddress::canonical::<TestLayout>(0xffff_8000_0000_0000),
            Some(TestLayout::KERNEL_HALF_START)
        );
        assert!(VirtualAddress::canonical::<TestLayout>(u64::MAX).is_some());
    }

    #[test]
    fn canonical_rejects_hole() {
        assert_eq!(
            VirtualAddress::canonical::<TestLayout>(0x0000_8000_0000_0000),
            None
        );
        assert_eq!(
            VirtualAddress::canonical::<TestLayout>(0xffff_7fff_ffff_ffff),
            None
        );
    }

    #[test]
    fn within_accepts_the_last_physical_address() {
        assert_eq!(
            PhysicalAddress::within::<TestLayout>(0x000f_ffff_ffff_ffff),
            Some(PhysicalAddress::new(0x000f_ffff_ffff_ffff))
        );
    }

    #[test]
    fn within_rejects_beyond_physical_bits() {
        assert_eq!(
            PhysicalAddress::within::<TestLayout>(0x0010_0000_0000_0000),
            None
        );
    }

    enum FullWidthLayout {}

    impl AddressLayout for FullWidthLayout {
        const VIRTUAL_ADDRESS_BIT_COUNT: u32 = 64;
        const PHYSICAL_ADDRESS_BIT_COUNT: u32 = 64;
        const KERNEL_HALF_START: VirtualAddress = VirtualAddress::new(0x8000_0000_0000_0000);
        const PHYSICAL_MAP_BASE: VirtualAddress = VirtualAddress::new(0x8000_0000_0000_0000);
        const PHYSICAL_MAP_SIZE: u64 = 1 << 62;
    }

    #[test]
    fn full_width_layout_accepts_everything() {
        assert!(VirtualAddress::canonical::<FullWidthLayout>(0x0000_8000_0000_0000).is_some());
        assert!(PhysicalAddress::within::<FullWidthLayout>(u64::MAX).is_some());
    }
}
