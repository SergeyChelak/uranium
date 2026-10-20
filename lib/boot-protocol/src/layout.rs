/// Pins the size of a wire structure and the offset of each of its fields,
/// so that the layout the compiler produces for every target is the one the
/// kernel and the loader agreed on; every wire structure is aligned to eight
/// bytes
macro_rules! layout {
    ($type:ty, $size:expr, { $($field:ident: $offset:literal),+ $(,)? }) => {
        const _: () = {
            assert!(::core::mem::size_of::<$type>() == $size);
            assert!(::core::mem::align_of::<$type>() == 8);
            $(assert!(::core::mem::offset_of!($type, $field) == $offset);)+
        };
    };
}

pub(crate) use layout;
