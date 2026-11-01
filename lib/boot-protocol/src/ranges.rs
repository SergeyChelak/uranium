use core::num::NonZeroU64;

use address::PhysicalAddress;
use byte_fields::{layout, structure};

structure! {
    pub(crate) struct RawRange {
        pub(crate) base: PhysicalAddress,
        pub(crate) size: u64,
    }
}

layout!(RawRange, 16, { base: 0, size: 8 });

impl RawRange {
    pub(crate) const ABSENT: Self = Self {
        base: PhysicalAddress::new(0),
        size: 0,
    };
}

impl From<PhysicalRange> for RawRange {
    fn from(range: PhysicalRange) -> Self {
        Self {
            base: range.base,
            size: range.size.get(),
        }
    }
}

structure! {
    pub(crate) struct RawTable {
        pub(crate) base: PhysicalAddress,
        pub(crate) count: u64,
        pub(crate) entry_size: u64,
    }
}

layout!(RawTable, 24, { base: 0, count: 8, entry_size: 16 });

impl From<Table> for RawTable {
    fn from(table: Table) -> Self {
        Self {
            base: table.base,
            count: table.count,
            entry_size: table.entry_size,
        }
    }
}

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

/// Where a table of fixed-size entries lies in physical memory; the bytes
/// the entries occupy always fit in `u64`, so nobody checks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Table {
    base: PhysicalAddress,
    count: u64,
    entry_size: u64,
}

impl Table {
    /// `None` when `count × entry_size` does not fit in `u64`
    pub fn new(base: PhysicalAddress, count: u64, entry_size: u64) -> Option<Self> {
        count.checked_mul(entry_size).map(|_| Self {
            base,
            count,
            entry_size,
        })
    }

    pub(crate) fn from_raw(raw: &RawTable) -> Option<Self> {
        Self::new(raw.base, raw.count, raw.entry_size)
    }

    pub fn base(self) -> PhysicalAddress {
        self.base
    }

    pub fn count(self) -> u64 {
        self.count
    }

    pub fn entry_size(self) -> u64 {
        self.entry_size
    }

    /// The bytes the entries occupy
    pub fn size(self) -> u64 {
        self.count * self.entry_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_refuses_entries_that_do_not_fit_in_u64() {
        let base = PhysicalAddress::new(0x1000);
        assert!(Table::new(base, u64::MAX, 2).is_none());
        assert!(Table::new(base, 2, u64::MAX).is_none());
    }

    #[test]
    fn table_size_is_the_product_up_to_the_limit() {
        let base = PhysicalAddress::new(0x1000);
        assert_eq!(Table::new(base, 3, 24).unwrap().size(), 72);
        assert_eq!(Table::new(base, u64::MAX, 1).unwrap().size(), u64::MAX);
        assert_eq!(Table::new(base, 0, 48).unwrap().size(), 0);
    }
}
