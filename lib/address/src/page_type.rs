macro_rules! page_type {
    ($name:ident, $address:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name<S: $crate::PageSize = $crate::Size4KiB> {
            number: u64,
            size: core::marker::PhantomData<S>,
        }

        impl<S: $crate::PageSize> $name<S> {
            /// The one that holds `address`
            pub fn containing(address: $crate::$address) -> Self {
                Self {
                    number: address.as_u64() / S::SIZE,
                    size: core::marker::PhantomData,
                }
            }

            /// `None` unless `address` is the first byte of one
            pub fn from_start(address: $crate::$address) -> Option<Self> {
                address
                    .is_aligned(S::SIZE)
                    .then(|| Self::containing(address))
            }

            pub(crate) fn from_number(number: u64) -> Self {
                Self {
                    number,
                    size: core::marker::PhantomData,
                }
            }

            pub fn start_address(self) -> $crate::$address {
                $crate::$address::new(self.number * S::SIZE)
            }

            pub fn number(self) -> u64 {
                self.number
            }
        }

        impl<S: $crate::PageSize> core::fmt::Debug for $name<S> {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(
                    formatter,
                    "{}<{}>({:#018x})",
                    stringify!($name),
                    S::NAME,
                    self.start_address()
                )
            }
        }
    };
}

pub(crate) use page_type;
