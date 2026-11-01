#![no_std]

#[cfg(test)]
extern crate std;

mod builder;
mod framebuffer;
mod information;
mod memory_map;
mod ranges;
mod serial_console;

pub use builder::{BootInformationBuilder, BuildError};
pub use framebuffer::{Framebuffer, PixelFormat};
pub use information::{BootInformation, BootInformationError, RawBootInformation, TableTooShort};
pub use memory_map::{EntryTooShort, MemoryKind, MemoryRegion, UefiMemoryMap};
pub use ranges::{PhysicalRange, Table};
pub use serial_console::SerialConsole;
