use core::num::NonZeroU64;

use address::{PhysicalAddress, VirtualAddress};

use crate::{Decode, Decoder, Encode, Encoder, OutOfBounds};

macro_rules! integer {
    ($($type:ty),+) => {
        $(
            impl Decode for $type {
                fn decode(decoder: &mut Decoder<'_>) -> Result<Self, OutOfBounds> {
                    decoder.array().map(Self::from_le_bytes)
                }
            }

            impl Encode for $type {
                fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), OutOfBounds> {
                    encoder.bytes(&self.to_le_bytes())
                }
            }
        )+
    };
}

integer!(u8, u16, u32, u64);

impl<const N: usize> Decode for [u8; N] {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, OutOfBounds> {
        decoder.array()
    }
}

impl<const N: usize> Encode for [u8; N] {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), OutOfBounds> {
        encoder.bytes(self)
    }
}

impl Decode for PhysicalAddress {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, OutOfBounds> {
        decoder.read().map(Self::new)
    }
}

impl Encode for PhysicalAddress {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), OutOfBounds> {
        encoder.write(&self.as_u64())
    }
}

impl Decode for VirtualAddress {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, OutOfBounds> {
        decoder.read().map(Self::new)
    }
}

impl Encode for VirtualAddress {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), OutOfBounds> {
        encoder.write(&self.as_u64())
    }
}

/// Zero on the wire is `None`
impl Decode for Option<NonZeroU64> {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, OutOfBounds> {
        decoder.read().map(NonZeroU64::new)
    }
}

impl Encode for Option<NonZeroU64> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), OutOfBounds> {
        encoder.write(&self.map_or(0, NonZeroU64::get))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<T: Decode + Encode + PartialEq + core::fmt::Debug>(value: T) {
        let mut block = [0xaau8; 16];
        let mut encoder = Encoder::new(&mut block);
        encoder.write(&value).unwrap();
        let written = encoder.position();
        let mut decoder = Decoder::new(&block);
        assert_eq!(decoder.read::<T>().unwrap(), value);
        assert_eq!(decoder.position(), written);
    }

    #[test]
    fn every_primitive_round_trips() {
        round_trip(0x7fu8);
        round_trip(0x1234u16);
        round_trip(0x1234_5678u32);
        round_trip(0x0102_0304_0506_0708u64);
        round_trip(*b"URANIUM0");
        round_trip(PhysicalAddress::new(0x5000_0000));
        round_trip(VirtualAddress::new(0xffff_8000_0000_0000));
        round_trip(NonZeroU64::new(0x7f00_0000));
        round_trip(None::<NonZeroU64>);
    }

    #[test]
    fn zero_decodes_as_none() {
        let mut decoder = Decoder::new(&[0u8; 8]);
        assert_eq!(decoder.read::<Option<NonZeroU64>>(), Ok(None));
    }
}
