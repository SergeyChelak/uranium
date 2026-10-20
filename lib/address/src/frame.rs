crate::page_type::page_type!(Frame, PhysicalAddress);

#[cfg(test)]
mod tests {
    use std::format;

    use super::Frame;
    use crate::{PhysicalAddress, Size2MiB, Size4KiB};

    #[test]
    fn containing_rounds_down() {
        let frame = Frame::<Size4KiB>::containing(PhysicalAddress::new(0x7fff_0abc));
        assert_eq!(frame.number(), 0x7fff0);
        assert_eq!(frame.start_address(), PhysicalAddress::new(0x7fff_0000));
    }

    #[test]
    fn from_start_requires_alignment() {
        assert_eq!(
            Frame::<Size2MiB>::from_start(PhysicalAddress::new(0x20_0000)),
            Some(Frame::containing(PhysicalAddress::new(0x20_0000)))
        );
        assert_eq!(
            Frame::<Size2MiB>::from_start(PhysicalAddress::new(0x10_0000)),
            None
        );
    }

    #[test]
    fn debug_names_the_size_and_the_start() {
        let frame = Frame::<Size4KiB>::containing(PhysicalAddress::new(0x1fff));
        assert_eq!(format!("{frame:?}"), "Frame<4KiB>(0x0000000000001000)");
    }
}
