//! Typed fields at fixed offsets in a byte slice: how the loader and the
//! kernel read tables and files whose layout a specification fixed outside
//! Rust, and how the loader writes the one format of its own. Every number
//! is little-endian. `Fields` reads at the offsets a specification names;
//! `Decoder` and `Encoder` walk fields in order, which is the layout of a
//! `#[repr(C)]` structure whose padding is spelled out; `layout!` pins such
//! a structure to the numbers of its document.

#![no_std]

#[cfg(test)]
extern crate std;

mod decoder;
mod encoder;
mod fields;
mod layout;
mod primitives;

pub use decoder::{Decode, Decoder};
pub use encoder::{Encode, Encoder};
pub use fields::Fields;

/// A field that reaches past the end of the bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfBounds;
