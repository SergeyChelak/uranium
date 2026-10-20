mod sealed {
    pub trait Sealed {}
}

/// A page size as a type: pages and frames of different sizes never mix, and
/// only the sizes listed here exist
pub trait PageSize: sealed::Sealed + Copy + Ord + core::hash::Hash {
    const SIZE: u64;
    const NAME: &'static str;
}

macro_rules! page_size {
    ($name:ident, $size:expr, $label:literal) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub enum $name {}

        impl sealed::Sealed for $name {}

        impl PageSize for $name {
            const SIZE: u64 = $size;
            const NAME: &'static str = $label;
        }
    };
}

page_size!(Size4KiB, 4 << 10, "4KiB");
page_size!(Size2MiB, 2 << 20, "2MiB");
page_size!(Size1GiB, 1 << 30, "1GiB");

#[cfg(test)]
mod tests {
    use super::{PageSize, Size1GiB, Size2MiB, Size4KiB};

    #[test]
    fn sizes_are_powers_of_two_in_bytes() {
        assert_eq!(Size4KiB::SIZE, 0x1000);
        assert_eq!(Size2MiB::SIZE, 0x20_0000);
        assert_eq!(Size1GiB::SIZE, 0x4000_0000);
    }

    #[test]
    fn larger_sizes_are_multiples_of_the_smallest() {
        assert_eq!(Size2MiB::SIZE % Size4KiB::SIZE, 0);
        assert_eq!(Size1GiB::SIZE % Size2MiB::SIZE, 0);
    }
}
