//! Physical and virtual addresses as two distinct types, so that the compiler
//! rejects every attempt to mix them.
//!
//! A physical address is never accepted where a virtual one is expected:
//!
//! ```compile_fail,E0308
//! fn takes_virtual(_address: address::VirtualAddress) {}
//! takes_virtual(address::PhysicalAddress::new(0));
//! ```
//!
//! A virtual address is never accepted where a physical one is expected:
//!
//! ```compile_fail,E0308
//! fn takes_physical(_address: address::PhysicalAddress) {}
//! takes_physical(address::VirtualAddress::new(0));
//! ```
//!
//! An address moves by an offset, but two addresses never add up:
//!
//! ```compile_fail,E0308
//! let _ = address::PhysicalAddress::new(0) + address::PhysicalAddress::new(0);
//! ```
//!
//! A raw number becomes an address only through an explicit constructor,
//! never through an implicit conversion:
//!
//! ```compile_fail,E0277
//! let _address: address::PhysicalAddress = 0u64.into();
//! ```

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
