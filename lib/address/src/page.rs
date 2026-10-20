crate::page_type::page_type!(Page, VirtualAddress);

#[cfg(test)]
mod tests {
    use std::format;

    use super::Page;
    use crate::{PageSize, Size1GiB, Size2MiB, Size4KiB, VirtualAddress};

    #[test]
    fn containing_rounds_down() {
        let page = Page::<Size4KiB>::containing(VirtualAddress::new(0x1234));
        assert_eq!(page.number(), 1);
        assert_eq!(page.start_address(), VirtualAddress::new(0x1000));
    }

    #[test]
    fn from_start_requires_alignment() {
        assert_eq!(
            Page::<Size4KiB>::from_start(VirtualAddress::new(0x2000)),
            Some(Page::containing(VirtualAddress::new(0x2000)))
        );
        assert_eq!(
            Page::<Size4KiB>::from_start(VirtualAddress::new(0x2008)),
            None
        );
    }

    #[test]
    fn large_page_start_is_aligned() {
        let large = Page::<Size2MiB>::containing(VirtualAddress::new(0x1234_5678));
        assert!(large.start_address().is_aligned(Size2MiB::SIZE));
        assert_eq!(large.start_address(), VirtualAddress::new(0x1220_0000));
        let huge = Page::<Size1GiB>::containing(VirtualAddress::new(0x1234_5678));
        assert_eq!(huge.start_address(), VirtualAddress::new(0));
    }

    #[test]
    fn default_size_is_four_kib() {
        let page: Page = Page::containing(VirtualAddress::new(0x3000));
        assert_eq!(
            page,
            Page::<Size4KiB>::containing(VirtualAddress::new(0x3000))
        );
    }

    #[test]
    fn orders_by_number() {
        let lower = Page::<Size4KiB>::containing(VirtualAddress::new(0x1000));
        let upper = Page::<Size4KiB>::containing(VirtualAddress::new(0x2000));
        assert!(lower < upper);
    }

    #[test]
    fn debug_names_the_size_and_the_start() {
        let page = Page::<Size2MiB>::containing(VirtualAddress::new(0xffff_ffff_8010_0000));
        assert_eq!(format!("{page:?}"), "Page<2MiB>(0xffffffff80000000)");
    }
}
