#![no_std]

mod entry;
mod layout;

pub use address::PhysicalAddress;
pub use layout::Layout;

pub enum Aarch64 {}

impl hal::Hal for Aarch64 {
    type Layout = Layout;
}
