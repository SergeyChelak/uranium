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
//!
//! Pages of different sizes never mix:
//!
//! ```compile_fail,E0308
//! let small = address::Page::<address::Size4KiB>::containing(address::VirtualAddress::new(0));
//! let large = address::Page::<address::Size2MiB>::containing(address::VirtualAddress::new(0));
//! let _ = small == large;
//! ```
//!
//! A page is never built from a physical address, nor a frame from a virtual
//! one; the link between them is a mapping, which lives elsewhere:
//!
//! ```compile_fail,E0308
//! let _ = address::Page::<address::Size4KiB>::containing(address::PhysicalAddress::new(0));
//! ```

#![no_std]

#[cfg(test)]
extern crate std;

mod address_type;
mod alignment;
mod frame;
mod layout;
mod page;
mod page_size;
mod page_type;
mod physical_address;
mod virtual_address;

pub use frame::Frame;
pub use layout::AddressLayout;
pub use page::Page;
pub use page_size::{PageSize, Size1GiB, Size2MiB, Size4KiB};
pub use physical_address::PhysicalAddress;
pub use virtual_address::VirtualAddress;
