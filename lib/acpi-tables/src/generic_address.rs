use crate::AcpiError;
use crate::fields::Fields;

// ACPI 6.6, 5.2.3.2
const ADDRESS_SPACE_OFFSET: usize = 0;
const RAW_VALUE_OFFSET: usize = 4;
const SYSTEM_MEMORY_SPACE: u8 = 0;
const SYSTEM_IO_SPACE: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpace {
    Memory,
    Io,
    Other(u8),
}

/// A register location as the table encodes it: a number whose meaning the
/// address space decides, a physical address for memory, a port number for I/O
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericAddress {
    pub space: AddressSpace,
    pub(crate) raw_value: u64,
}

impl GenericAddress {
    // ACPI 6.6, 5.2.3.2: the address field ends at byte 12
    pub const LENGTH: usize = 12;

    pub fn parse(bytes: &[u8]) -> Result<Self, AcpiError> {
        let fields = Fields::exactly(bytes, Self::LENGTH)?;
        let space = match fields.u8(ADDRESS_SPACE_OFFSET)? {
            SYSTEM_MEMORY_SPACE => AddressSpace::Memory,
            SYSTEM_IO_SPACE => AddressSpace::Io,
            other => AddressSpace::Other(other),
        };
        Ok(Self {
            space,
            raw_value: fields.u64(RAW_VALUE_OFFSET)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{AddressSpace, GenericAddress};
    use crate::AcpiError;

    fn generic_address(space: u8, raw_value: u64) -> [u8; 12] {
        let mut bytes = [0u8; 12];
        bytes[0] = space;
        bytes[4..].copy_from_slice(&raw_value.to_le_bytes());
        bytes
    }

    #[test]
    fn generic_address_reads_memory_space() {
        assert_eq!(
            GenericAddress::parse(&generic_address(0, 0x0900_0000)),
            Ok(GenericAddress {
                space: AddressSpace::Memory,
                raw_value: 0x0900_0000,
            })
        );
    }

    #[test]
    fn generic_address_reads_io_space() {
        assert_eq!(
            GenericAddress::parse(&generic_address(1, 0x3f8)),
            Ok(GenericAddress {
                space: AddressSpace::Io,
                raw_value: 0x3f8,
            })
        );
    }

    #[test]
    fn generic_address_keeps_an_unknown_space() {
        assert_eq!(
            GenericAddress::parse(&generic_address(2, 0)).map(|generic| generic.space),
            Ok(AddressSpace::Other(2))
        );
    }

    #[test]
    fn generic_address_rejects_a_short_buffer() {
        assert_eq!(
            GenericAddress::parse(&generic_address(0, 0)[..11]),
            Err(AcpiError::Length)
        );
    }
}
