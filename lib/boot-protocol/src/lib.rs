//! The contract between the loader and the kernel: what the loader leaves in
//! memory for the kernel to find at its entry point, and how the kernel reads
//! it. The wire format is a plain `#[repr(C)]` structure of integers whose
//! layout is pinned by compile-time checks and stays inside this crate; the
//! kernel sees it through `BootInformation`, which checks the block once and
//! then answers in typed values, with `None` where the loader had nothing to
//! say and a safe default where a code is newer than this kernel. The loader
//! fills the block through `BootInformationBuilder`, the only other place
//! that knows how the typed values are encoded.

#![no_std]

#[cfg(test)]
extern crate std;

mod builder;
mod fields;
mod framebuffer;
mod information;
mod layout;
mod memory_map;
mod ranges;
mod serial_console;

pub use builder::{BootInformationBuilder, BuildError};
pub use framebuffer::{Framebuffer, PixelFormat};
pub use information::{BootInformation, BootInformationError, RawBootInformation};
pub use memory_map::{MemoryKind, MemoryRegion, UefiMemoryMap};
pub use ranges::{PhysicalRange, Table};
pub use serial_console::{SerialConsole, SerialKind, UnknownSerialKind};
