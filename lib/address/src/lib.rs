#![no_std]

#[cfg(test)]
extern crate std;

mod address_type;
mod alignment;
mod physical_address;
mod virtual_address;

pub use physical_address::PhysicalAddress;
pub use virtual_address::VirtualAddress;
