#![no_std]

#[cfg(test)]
extern crate std;

mod serial_console;
mod text_console;
mod uefi_firmware;
mod volume;

pub use serial_console::SerialConsole;
pub use text_console::TextConsole;
pub use uefi_firmware::UefiFirmware;
pub use volume::{UefiFile, UefiVolume};
