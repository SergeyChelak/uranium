/// Pins the size of a `#[repr(C)]` structure and the offset of each of its
/// fields to the numbers its document states, so that the layout the
/// compiler produces for every target is the agreed one. A structure is
/// aligned to eight bytes unless the layout names another alignment, as one
/// made of bytes alone (`align 1`) does
#[macro_export]
macro_rules! layout {
    ($type:ty, $size:expr, { $($field:ident: $offset:literal),+ $(,)? }) => {
        $crate::layout!($type, $size, align 8, { $($field: $offset),+ });
    };
    ($type:ty, $size:expr, align $align:literal, { $($field:ident: $offset:literal),+ $(,)? }) => {
        const _: () = {
            assert!(::core::mem::size_of::<$type>() == $size);
            assert!(::core::mem::align_of::<$type>() == $align);
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

    #[repr(C)]
    struct Bytes {
        first: [u8; 4],
        second: u8,
    }

    crate::layout!(Bytes, 5, align 1, { first: 0, second: 4 });

    #[test]
    fn layout_pins_a_structure() {
        assert_eq!(core::mem::size_of::<Pair>(), 16);
        assert_eq!(core::mem::size_of::<Bytes>(), 5);
    }
}
