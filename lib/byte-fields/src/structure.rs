/// A `#[repr(C)]` structure whose fields are written and read in the order
/// they are declared: one list of fields is the structure, its encoding and
/// its decoding, so no field can be forgotten or reordered on one side only.
///
/// ```
/// byte_fields::structure! {
///     /// Attributes and doc comments pass through
///     #[derive(Debug, PartialEq)]
///     struct Header {
///         tag: u16,
///         padding: u16,
///         length: u32,
///     }
/// }
///
/// let header = Header { tag: 1, padding: 0, length: 2 };
/// let mut block = [0u8; 8];
/// byte_fields::Encoder::new(&mut block).write(&header).unwrap();
/// assert_eq!(block, [1, 0, 0, 0, 2, 0, 0, 0]);
/// assert_eq!(byte_fields::Decoder::new(&block).read(), Ok(header));
/// ```
///
/// The invocation above expands to the structure itself and to its two
/// codings, nothing else:
///
/// ```text
/// /// Attributes and doc comments pass through
/// #[derive(Debug, PartialEq)]
/// #[repr(C)]
/// struct Header {
///     tag: u16,
///     padding: u16,
///     length: u32,
/// }
///
/// impl byte_fields::Encode for Header {
///     fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), OutOfBounds> {
///         encoder.write(&self.tag)?;
///         encoder.write(&self.padding)?;
///         encoder.write(&self.length)?;
///         Ok(())
///     }
/// }
///
/// impl byte_fields::Decode for Header {
///     fn decode(decoder: &mut Decoder<'_>) -> Result<Self, OutOfBounds> {
///         Ok(Self {
///             tag: decoder.read()?,
///             padding: decoder.read()?,
///             length: decoder.read()?,
///         })
///     }
/// }
/// ```
///
/// Every field type must implement `Encode` and `Decode`: the integers, byte
/// arrays, addresses and `Option<NonZeroU64>` do, and so does any structure
/// declared this way, which is how structures nest. The layout is the
/// declaration order under natural alignment, so the padding a document
/// leaves between fields is spelled out as a field, as `padding` is above;
/// a packed layout cannot be declared this way and is read through `Fields`
/// at the offsets the document names. `layout!` pins the result to the
/// document's numbers.
#[macro_export]
macro_rules! structure {
    (
        $(#[$attribute:meta])*
        $visibility:vis struct $name:ident {
            $(
                $(#[$field_attribute:meta])*
                $field_visibility:vis $field:ident: $type:ty
            ),+ $(,)?
        }
    ) => {
        $(#[$attribute])*
        #[repr(C)]
        $visibility struct $name {
            $(
                $(#[$field_attribute])*
                $field_visibility $field: $type,
            )+
        }

        impl $crate::Encode for $name {
            fn encode(
                &self,
                encoder: &mut $crate::Encoder<'_>,
            ) -> Result<(), $crate::OutOfBounds> {
                $(encoder.write(&self.$field)?;)+
                Ok(())
            }
        }

        impl $crate::Decode for $name {
            fn decode(decoder: &mut $crate::Decoder<'_>) -> Result<Self, $crate::OutOfBounds> {
                Ok(Self {
                    $($field: decoder.read()?,)+
                })
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::{Decoder, Encoder, Fields, OutOfBounds};

    crate::structure! {
        /// A header with one padding field
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct Header {
            tag: u16,
            padding: u16,
            length: u32,
            /// Where the body starts
            body: u64,
        }
    }

    crate::layout!(Header, 16, { tag: 0, padding: 2, length: 4, body: 8 });

    const HEADER: Header = Header {
        tag: 0x1234,
        padding: 0,
        length: 0x0a0b_0c0d,
        body: 0x0102_0304_0506_0708,
    };

    #[test]
    fn a_structure_round_trips() {
        let mut block = [0u8; 16];
        Encoder::new(&mut block).write(&HEADER).unwrap();
        assert_eq!(Decoder::new(&block).read::<Header>(), Ok(HEADER));
    }

    #[test]
    fn fields_are_written_in_declaration_order() {
        let mut block = [0u8; 16];
        Encoder::new(&mut block).write(&HEADER).unwrap();
        let fields = Fields::new(&block);
        assert_eq!(fields.u16(0), Ok(0x1234));
        assert_eq!(fields.u16(2), Ok(0));
        assert_eq!(fields.u32(4), Ok(0x0a0b_0c0d));
        assert_eq!(fields.u64(8), Ok(0x0102_0304_0506_0708));
    }

    #[test]
    fn a_structure_that_does_not_fit_is_out_of_bounds() {
        let mut block = [0u8; 15];
        assert_eq!(Encoder::new(&mut block).write(&HEADER), Err(OutOfBounds));
        assert_eq!(Decoder::new(&block).read::<Header>(), Err(OutOfBounds));
    }
}
