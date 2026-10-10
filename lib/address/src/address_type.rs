macro_rules! address_type {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name(u64);

        impl $name {
            pub const fn new(raw: u64) -> Self {
                Self(raw)
            }

            pub const fn as_u64(self) -> u64 {
                self.0
            }

            pub fn checked_add(self, offset: u64) -> Option<Self> {
                self.0.checked_add(offset).map(Self)
            }

            pub fn align_down(self, alignment: u64) -> Self {
                Self(crate::alignment::align_down(self.0, alignment))
            }

            pub fn align_up(self, alignment: u64) -> Self {
                Self(crate::alignment::align_up(self.0, alignment))
            }

            pub fn is_aligned(self, alignment: u64) -> bool {
                crate::alignment::is_aligned(self.0, alignment)
            }
        }

        impl core::ops::Add<u64> for $name {
            type Output = Self;

            fn add(self, offset: u64) -> Self {
                self.checked_add(offset)
                    .expect("address offset overflows the address space")
            }
        }

        impl core::ops::Sub for $name {
            type Output = u64;

            fn sub(self, other: Self) -> u64 {
                self.0
                    .checked_sub(other.0)
                    .expect("subtrahend address is above the minuend")
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(formatter, "{}({:#018x})", stringify!($name), self.0)
            }
        }

        impl core::fmt::LowerHex for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::LowerHex::fmt(&self.0, formatter)
            }
        }
    };
}

pub(crate) use address_type;
