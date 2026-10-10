#![no_std]

mod entry;
mod layout;

pub use address::PhysicalAddress;
pub use layout::Layout;

pub enum X86_64 {}

impl hal::Hal for X86_64 {
    type Layout = Layout;
}
