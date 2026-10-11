#![no_std]

#[cfg(test)]
extern crate std;

mod console;

pub use console::{Console, ConsoleError};
