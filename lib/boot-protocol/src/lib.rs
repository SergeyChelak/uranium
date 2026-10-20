//! The contract between the loader and the kernel: what the loader leaves in
//! memory for the kernel to find at its entry point. The wire format is a
//! plain `#[repr(C)]` structure of integers whose layout is pinned by
//! compile-time checks; the enumerations that travel in it as `u32` codes
//! decode so that a code the kernel does not know can never make it touch
//! memory it should not.

#![no_std]

mod kinds;
mod raw;

pub use kinds::{MemoryKind, PixelFormat, SerialKind, UnknownSerialKind};
pub use raw::RawBootInformation;
