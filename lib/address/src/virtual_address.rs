crate::address_type::address_type!(VirtualAddress);

impl VirtualAddress {
    pub fn from_pointer<T>(pointer: *const T) -> Self {
        Self::new(pointer.addr() as u64)
    }

    pub fn as_pointer<T>(self) -> *const T {
        core::ptr::with_exposed_provenance(self.as_usize())
    }

    pub fn as_mut_pointer<T>(self) -> *mut T {
        core::ptr::with_exposed_provenance_mut(self.as_usize())
    }

    fn as_usize(self) -> usize {
        usize::try_from(self.0).expect("virtual address fits in a pointer")
    }
}

#[cfg(test)]
mod tests {
    use std::format;

    use super::VirtualAddress;

    #[test]
    fn from_pointer_round_trips() {
        let value = 42u32;
        let address = VirtualAddress::from_pointer(&raw const value);
        assert_eq!(address.as_pointer::<u32>(), &raw const value);
    }

    #[test]
    fn as_mut_pointer_has_the_same_address() {
        let address = VirtualAddress::new(0x1000);
        assert_eq!(address.as_mut_pointer::<u8>().addr(), 0x1000);
    }

    #[test]
    fn shares_the_address_arithmetic() {
        let address = VirtualAddress::new(0x1234);
        assert_eq!(address + 0x10, VirtualAddress::new(0x1244));
        assert_eq!(address - VirtualAddress::new(0x1000), 0x234);
        assert_eq!(address.align_down(0x1000), VirtualAddress::new(0x1000));
        assert_eq!(address.align_up(0x1000), VirtualAddress::new(0x2000));
        assert!(!address.is_aligned(0x1000));
        assert_eq!(VirtualAddress::new(u64::MAX).checked_add(1), None);
    }

    #[test]
    fn debug_prints_hexadecimal() {
        assert_eq!(
            format!("{:?}", VirtualAddress::new(0xffff_8000_0000_0000)),
            "VirtualAddress(0xffff800000000000)"
        );
    }
}
