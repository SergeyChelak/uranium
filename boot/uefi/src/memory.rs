use core::mem::{offset_of, size_of};
use core::ptr;

use address::PhysicalAddress;

use crate::Status;

// UEFI 2.11, 7.2.1, EFI_MEMORY_TYPE: the specification fixes only the order
// of the variants; the values follow the C rules for an enum without initializers
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum MemoryType {
    Reserved = 0,
    LoaderCode = 1,
    LoaderData = 2,
    BootServicesCode = 3,
    BootServicesData = 4,
    RuntimeServicesCode = 5,
    RuntimeServicesData = 6,
    Conventional = 7,
    Unusable = 8,
    AcpiReclaim = 9,
    AcpiNonVolatileStorage = 10,
    MemoryMappedIo = 11,
    MemoryMappedIoPortSpace = 12,
    PalCode = 13,
    Persistent = 14,
    Unaccepted = 15,
}

const MEMORY_TYPES_IN_ORDER: [MemoryType; 16] = [
    MemoryType::Reserved,
    MemoryType::LoaderCode,
    MemoryType::LoaderData,
    MemoryType::BootServicesCode,
    MemoryType::BootServicesData,
    MemoryType::RuntimeServicesCode,
    MemoryType::RuntimeServicesData,
    MemoryType::Conventional,
    MemoryType::Unusable,
    MemoryType::AcpiReclaim,
    MemoryType::AcpiNonVolatileStorage,
    MemoryType::MemoryMappedIo,
    MemoryType::MemoryMappedIoPortSpace,
    MemoryType::PalCode,
    MemoryType::Persistent,
    MemoryType::Unaccepted,
];

impl TryFrom<u32> for MemoryType {
    /// The code the firmware used when it names no known type
    type Error = u32;

    fn try_from(code: u32) -> Result<Self, u32> {
        usize::try_from(code)
            .ok()
            .and_then(|index| MEMORY_TYPES_IN_ORDER.get(index))
            .copied()
            .ok_or(code)
    }
}

// UEFI 2.11, 7.2.1
pub const PAGE_SIZE: usize = 4096;

// UEFI 2.11, 7.2.1, EFI_ALLOCATE_TYPE: the values follow the C rules for an
// enum without initializers; declared up to the kind the loader uses
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub(crate) enum AllocateKind {
    AnyPages = 0,
}

// UEFI 2.11, 7.2.3
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryDescriptor {
    memory_type: u32,
    physical_start: u64,
    virtual_start: u64,
    page_count: u64,
    attribute: u64,
}

impl MemoryDescriptor {
    /// The type of the region, or the code the firmware used when it is not
    /// a known one
    pub fn memory_type(&self) -> Result<MemoryType, u32> {
        MemoryType::try_from(self.memory_type)
    }

    pub fn physical_start(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.physical_start)
    }

    pub fn page_count(&self) -> u64 {
        self.page_count
    }
}

/// Names the memory map at one moment; `ExitBootServices` takes it back and
/// refuses a key of a map that has changed since
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MemoryMapKey(pub(crate) usize);

/// A memory map the firmware wrote into a caller's buffer; entries are read
/// by `descriptor_size`, which a newer firmware may set wider than the
/// declared structure
pub struct MemoryMap<'a> {
    bytes: &'a [u8],
    descriptor_size: usize,
    descriptor_version: u32,
    key: MemoryMapKey,
}

impl<'a> MemoryMap<'a> {
    /// `DEVICE_ERROR` when the firmware's descriptor is narrower than the
    /// declared structure or the map does not hold whole descriptors
    pub(crate) fn new(
        bytes: &'a [u8],
        descriptor_size: usize,
        descriptor_version: u32,
        key: MemoryMapKey,
    ) -> Result<Self, Status> {
        if descriptor_size < size_of::<MemoryDescriptor>()
            || !bytes.len().is_multiple_of(descriptor_size)
        {
            return Err(Status::DEVICE_ERROR);
        }
        Ok(Self {
            bytes,
            descriptor_size,
            descriptor_version,
            key,
        })
    }

    pub fn key(&self) -> MemoryMapKey {
        self.key
    }

    pub fn descriptor_size(&self) -> usize {
        self.descriptor_size
    }

    pub fn descriptor_version(&self) -> u32 {
        self.descriptor_version
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub fn descriptors(&self) -> impl Iterator<Item = MemoryDescriptor> + 'a {
        self.bytes.chunks_exact(self.descriptor_size).map(|chunk| {
            // SAFETY: `new` checked that every chunk holds a whole descriptor,
            // and every bit pattern is valid for its integer fields
            unsafe { ptr::read_unaligned(chunk.as_ptr().cast::<MemoryDescriptor>()) }
        })
    }
}

const _: () = assert!(offset_of!(MemoryDescriptor, physical_start) == 8);
const _: () = assert!(offset_of!(MemoryDescriptor, virtual_start) == 16);
const _: () = assert!(offset_of!(MemoryDescriptor, page_count) == 24);
const _: () = assert!(offset_of!(MemoryDescriptor, attribute) == 32);
const _: () = assert!(size_of::<MemoryDescriptor>() == 40);

#[cfg(test)]
mod tests {
    use super::{MemoryDescriptor, MemoryMap, MemoryMapKey, MemoryType};
    use crate::Status;
    use address::PhysicalAddress;
    use core::mem::size_of;
    use std::vec;
    use std::vec::Vec;

    /// A descriptor wider than the structure, as a newer firmware may return
    const STRIDE: usize = size_of::<MemoryDescriptor>() + 8;

    fn descriptor(memory_type: u32, physical_start: u64, page_count: u64) -> MemoryDescriptor {
        MemoryDescriptor {
            memory_type,
            physical_start,
            virtual_start: 0,
            page_count,
            attribute: 0,
        }
    }

    fn raw_map(descriptors: &[MemoryDescriptor]) -> Vec<u8> {
        let mut bytes = vec![0; descriptors.len() * STRIDE];
        for (slot, descriptor) in bytes.chunks_exact_mut(STRIDE).zip(descriptors) {
            // SAFETY: `slot` holds `STRIDE` bytes, more than a descriptor needs
            unsafe {
                slot.as_mut_ptr()
                    .cast::<MemoryDescriptor>()
                    .write_unaligned(*descriptor);
            }
        }
        bytes
    }

    #[test]
    fn descriptors_step_by_descriptor_size() {
        let bytes = raw_map(&[descriptor(7, 0x1000, 2), descriptor(2, 0x8000, 1)]);
        let map = MemoryMap::new(&bytes, STRIDE, 1, MemoryMapKey(7)).unwrap();
        let descriptors: Vec<MemoryDescriptor> = map.descriptors().collect();
        assert_eq!(descriptors.len(), 2);
        assert_eq!(descriptors[0].memory_type(), Ok(MemoryType::Conventional));
        assert_eq!(
            descriptors[0].physical_start(),
            PhysicalAddress::new(0x1000)
        );
        assert_eq!(descriptors[0].page_count(), 2);
        assert_eq!(descriptors[1].memory_type(), Ok(MemoryType::LoaderData));
        assert_eq!(
            descriptors[1].physical_start(),
            PhysicalAddress::new(0x8000)
        );
        assert_eq!(descriptors[1].page_count(), 1);
        assert_eq!(map.key(), MemoryMapKey(7));
        assert_eq!(map.descriptor_size(), STRIDE);
        assert_eq!(map.descriptor_version(), 1);
        assert_eq!(map.bytes(), &bytes[..]);
    }

    #[test]
    fn unknown_memory_type_is_reported_with_its_code() {
        assert_eq!(descriptor(16, 0, 1).memory_type(), Err(16));
        assert_eq!(
            descriptor(0x8000_0000, 0, 1).memory_type(),
            Err(0x8000_0000)
        );
    }

    #[test]
    fn memory_types_follow_the_order_of_the_specification() {
        assert_eq!(MemoryType::try_from(0), Ok(MemoryType::Reserved));
        assert_eq!(MemoryType::try_from(2), Ok(MemoryType::LoaderData));
        assert_eq!(MemoryType::try_from(7), Ok(MemoryType::Conventional));
        assert_eq!(MemoryType::try_from(15), Ok(MemoryType::Unaccepted));
        assert_eq!(MemoryType::AcpiNonVolatileStorage as u32, 10);
    }

    #[test]
    fn a_descriptor_narrower_than_declared_is_rejected() {
        let bytes = raw_map(&[descriptor(7, 0x1000, 2)]);
        let narrow = size_of::<MemoryDescriptor>() - 8;
        assert_eq!(
            MemoryMap::new(&bytes, narrow, 1, MemoryMapKey(1)).err(),
            Some(Status::DEVICE_ERROR)
        );
    }

    #[test]
    fn a_map_with_a_partial_descriptor_is_rejected() {
        let bytes = raw_map(&[descriptor(7, 0x1000, 2)]);
        assert_eq!(
            MemoryMap::new(&bytes[..STRIDE - 1], STRIDE, 1, MemoryMapKey(1)).err(),
            Some(Status::DEVICE_ERROR)
        );
    }
}
