#![no_std]

#[cfg(test)]
extern crate std;

mod decoder;
mod encoder;
mod fields;
mod layout;
mod primitives;
mod structure;

pub use decoder::{Decode, Decoder};
pub use encoder::{Encode, Encoder};
pub use fields::Fields;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfBounds;
