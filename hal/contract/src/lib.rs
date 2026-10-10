#![no_std]

pub trait Hal {
    type Layout: address::AddressLayout;
}
