//! The way from the root pointer to any other table: RSDP names the XSDT, the
//! XSDT lists every table, and each table starts with the header that names it

use address::PhysicalAddress;

use crate::{AcpiError, Rsdp, TableHeader, Xsdt};

/// Where the bytes of a firmware table come from: identity-mapped memory in
/// the loader, the physical window in the kernel
pub trait TableReader {
    /// `length` bytes of the table at `address`; `Address` when the range is
    /// not readable firmware memory
    fn bytes(&self, address: PhysicalAddress, length: usize) -> Result<&[u8], AcpiError>;
}

/// The table carrying `signature`, cut to the length its header claims; its
/// checksum is left to the parser of that table
pub(crate) fn table_with_signature<'r, R: TableReader>(
    reader: &'r R,
    rsdp: PhysicalAddress,
    signature: &[u8; 4],
) -> Result<&'r [u8], AcpiError> {
    let rsdp = Rsdp::parse(reader.bytes(rsdp, Rsdp::LENGTH)?)?;
    let xsdt = table_at(reader, rsdp.xsdt, &header_at(reader, rsdp.xsdt)?)?;
    for entry in Xsdt::entries(xsdt)? {
        let header = header_at(reader, entry)?;
        if header.signature == *signature {
            return table_at(reader, entry, &header);
        }
    }
    Err(AcpiError::NotFound)
}

fn header_at<R: TableReader>(
    reader: &R,
    address: PhysicalAddress,
) -> Result<TableHeader, AcpiError> {
    TableHeader::parse(reader.bytes(address, TableHeader::LENGTH)?)
}

fn table_at<'r, R: TableReader>(
    reader: &'r R,
    address: PhysicalAddress,
    header: &TableHeader,
) -> Result<&'r [u8], AcpiError> {
    let length = usize::try_from(header.length).map_err(|_| AcpiError::Length)?;
    reader.bytes(address, length)
}

#[cfg(test)]
pub(crate) mod testing {
    use std::collections::BTreeMap;
    use std::vec::Vec;

    use address::PhysicalAddress;

    use super::TableReader;
    use crate::AcpiError;
    use crate::header::testing::table;

    /// Firmware memory with a table at each address: a slice of the requested
    /// length from a table, `Address` for any other range
    pub(crate) struct Tables(pub(crate) BTreeMap<PhysicalAddress, Vec<u8>>);

    impl TableReader for Tables {
        fn bytes(&self, address: PhysicalAddress, length: usize) -> Result<&[u8], AcpiError> {
            self.0
                .get(&address)
                .and_then(|table| table.get(..length))
                .ok_or(AcpiError::Address)
        }
    }

    pub(crate) fn xsdt(entries: &[PhysicalAddress]) -> Vec<u8> {
        let body: Vec<u8> = entries
            .iter()
            .flat_map(|entry| entry.as_u64().to_le_bytes())
            .collect();
        table(b"XSDT", 1, &body)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use address::PhysicalAddress;

    use super::table_with_signature;
    use super::testing::{Tables, xsdt};
    use crate::AcpiError;
    use crate::header::testing::table;
    use crate::rsdp::testing::rsdp;

    const RSDP: PhysicalAddress = PhysicalAddress::new(0x1000);
    const XSDT: PhysicalAddress = PhysicalAddress::new(0x2000);
    const APIC: PhysicalAddress = PhysicalAddress::new(0x3000);
    const SPCR: PhysicalAddress = PhysicalAddress::new(0x4000);

    fn machine(entries: &[PhysicalAddress]) -> Tables {
        let mut tables = BTreeMap::new();
        tables.insert(RSDP, rsdp(2, 36, XSDT.as_u64()));
        tables.insert(XSDT, xsdt(entries));
        tables.insert(APIC, table(b"APIC", 5, &[0; 8]));
        tables.insert(SPCR, table(b"SPCR", 2, &[3; 16]));
        Tables(tables)
    }

    #[test]
    fn finds_spcr_through_rsdp_and_xsdt() {
        let tables = machine(&[APIC, SPCR]);
        assert_eq!(
            table_with_signature(&tables, RSDP, b"SPCR"),
            Ok(tables.0[&SPCR].as_slice())
        );
    }

    #[test]
    fn reports_a_missing_table() {
        let tables = machine(&[APIC]);
        assert_eq!(
            table_with_signature(&tables, RSDP, b"SPCR"),
            Err(AcpiError::NotFound)
        );
    }

    #[test]
    fn rejects_a_broken_xsdt_checksum() {
        let mut tables = machine(&[APIC, SPCR]);
        tables.0.get_mut(&XSDT).unwrap()[40] ^= 1;
        assert_eq!(
            table_with_signature(&tables, RSDP, b"SPCR"),
            Err(AcpiError::Checksum)
        );
    }

    #[test]
    fn rejects_an_unreadable_rsdp() {
        let tables = machine(&[APIC, SPCR]);
        assert_eq!(
            table_with_signature(&tables, PhysicalAddress::new(0x9000), b"SPCR"),
            Err(AcpiError::Address)
        );
    }

    #[test]
    fn rejects_an_entry_that_leads_nowhere() {
        let tables = machine(&[PhysicalAddress::new(0x5000), SPCR]);
        assert_eq!(
            table_with_signature(&tables, RSDP, b"SPCR"),
            Err(AcpiError::Address)
        );
    }
}
