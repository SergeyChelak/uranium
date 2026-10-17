#![no_std]
#![expect(
    unsafe_code,
    reason = "turns the pages the firmware allocates into slices and ends boot services"
)]

#[cfg(test)]
extern crate std;

mod exit;
mod memory;
mod platform;
mod serial_console;
mod text_console;
mod uefi_firmware;
mod volume;

pub use memory::UefiMemory;
pub use platform::UefiPlatform;
pub use serial_console::SerialConsole;
pub use text_console::TextConsole;
pub use uefi_firmware::UefiFirmware;
pub use volume::{UefiFile, UefiVolume};
