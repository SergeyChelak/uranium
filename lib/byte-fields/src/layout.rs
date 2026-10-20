/// Pins the size of a `#[repr(C)]` structure and the offset of each of its
/// fields to the numbers its document states, so that the layout the
/// compiler produces for every target is the agreed one; every such
/// structure is aligned to eight bytes
#[macro_export]
macro_rules! layout {
    ($type:ty, $size:expr, { $($field:ident: $offset:literal),+ $(,)? }) => {
        const _: () = {
            assert!(::core::mem::size_of::<$type>() == $size);
            assert!(::core::mem::align_of::<$type>() == 8);
            $(assert!(::core::mem::offset_of!($type, $field) == $offset);)+
        };
    };
}

#[cfg(test)]
mod tests {
    #[repr(C)]
    struct Pair {
        first: u64,
        second: u32,
        padding: u32,
    }

    crate::layout!(Pair, 16, { first: 0, second: 8, padding: 12 });

    #[test]
    fn layout_pins_a_structure() {
        assert_eq!(core::mem::size_of::<Pair>(), 16);
    }
}
