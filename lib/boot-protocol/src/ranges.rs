use core::num::NonZeroU64;

use address::PhysicalAddress;

use crate::layout::layout;

#[repr(C)]
pub(crate) struct RawRange {
    pub(crate) base: PhysicalAddress,
    pub(crate) size: u64,
}

layout!(RawRange, 16, { base: 0, size: 8 });

#[repr(C)]
pub(crate) struct RawTable {
    pub(crate) base: PhysicalAddress,
    pub(crate) count: u64,
    pub(crate) entry_size: u64,
}

layout!(RawTable, 24, { base: 0, count: 8, entry_size: 16 });

/// A region of physical memory; it cannot be empty, so nobody checks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalRange {
    pub base: PhysicalAddress,
    pub size: NonZeroU64,
}

impl PhysicalRange {
    pub fn new(base: PhysicalAddress, size: u64) -> Option<Self> {
        NonZeroU64::new(size).map(|size| Self { base, size })
    }

    pub(crate) fn from_raw(raw: &RawRange) -> Option<Self> {
        Self::new(raw.base, raw.size)
    }
}

/// Where a table of fixed-size entries lies in physical memory
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Table {
    pub base: PhysicalAddress,
    pub count: u64,
    pub entry_size: u64,
}

impl Table {
    pub(crate) fn from_raw(raw: &RawTable) -> Self {
        Self {
            base: raw.base,
            count: raw.count,
            entry_size: raw.entry_size,
        }
    }

    /// The bytes the entries occupy
    pub fn size(self) -> u64 {
        self.count.saturating_mul(self.entry_size)
    }
}
