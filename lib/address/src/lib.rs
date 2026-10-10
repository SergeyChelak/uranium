#![no_std]

#[cfg(test)]
extern crate std;

mod address_type;
mod alignment;
mod layout;
mod physical_address;
mod virtual_address;

pub use layout::AddressLayout;
pub use physical_address::PhysicalAddress;
pub use virtual_address::VirtualAddress;
