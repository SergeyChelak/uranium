use core::mem::{align_of, offset_of, size_of};
use core::num::NonZeroU64;

use address::{PhysicalAddress, VirtualAddress};

/// What the loader writes into memory and the kernel finds through the
/// register of its first argument
#[repr(C)]
pub struct RawBootInformation {
    pub(crate) magic: u64,
    pub(crate) version: u32,
    pub(crate) size: u32,
    pub(crate) physical_map_base: VirtualAddress,
    pub(crate) kernel_image: RawRange,
    pub(crate) kernel_virtual_base: VirtualAddress,
    pub(crate) kernel_stack: RawRange,
    pub(crate) page_tables: RawRange,
    pub(crate) loader_data: RawRange,
    pub(crate) memory_map: RawTable,
    pub(crate) uefi_memory_map: RawTable,
    pub(crate) uefi_descriptor_version: u32,
    pub(crate) reserved: u32,
    pub(crate) uefi_system_table: Option<NonZeroU64>,
    pub(crate) rsdp: Option<NonZeroU64>,
    pub(crate) device_tree: RawRange,
    pub(crate) framebuffer: RawFramebuffer,
    pub(crate) command_line: RawRange,
    pub(crate) initial_ramdisk: RawRange,
    pub(crate) serial_console: RawSerialConsole,
}

#[repr(C)]
pub(crate) struct RawRange {
    pub(crate) base: PhysicalAddress,
    pub(crate) size: u64,
}

#[repr(C)]
pub(crate) struct RawTable {
    pub(crate) base: PhysicalAddress,
    pub(crate) count: u64,
    pub(crate) entry_size: u64,
}

#[repr(C)]
pub(crate) struct RawMemoryRegion {
    pub(crate) base: PhysicalAddress,
    pub(crate) size: u64,
    pub(crate) kind: u32,
    pub(crate) reserved: u32,
}

#[repr(C)]
pub(crate) struct RawFramebuffer {
    pub(crate) base: PhysicalAddress,
    pub(crate) size: u64,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) stride: u32,
    pub(crate) pixel_format: u32,
}

/// `base` is a port number for a 16550 and a physical address for a PL011,
/// so the kind decides what it is
#[repr(C)]
pub(crate) struct RawSerialConsole {
    pub(crate) base: u64,
    pub(crate) kind: u32,
    pub(crate) reserved: u32,
}

/// Pins the size of a structure and the offset of each of its fields, so that
/// the layout the compiler produces for every target is the one the kernel
/// and the loader agreed on; every structure is aligned to eight bytes
macro_rules! layout {
    ($type:ty, $size:literal, { $($field:ident: $offset:literal),+ $(,)? }) => {
        const _: () = {
            assert!(size_of::<$type>() == $size);
            assert!(align_of::<$type>() == 8);
            $(assert!(offset_of!($type, $field) == $offset);)+
        };
    };
}

layout!(RawBootInformation, 264, {
    magic: 0,
    version: 8,
    size: 12,
    physical_map_base: 16,
    kernel_image: 24,
    kernel_virtual_base: 40,
    kernel_stack: 48,
    page_tables: 64,
    loader_data: 80,
    memory_map: 96,
    uefi_memory_map: 120,
    uefi_descriptor_version: 144,
    reserved: 148,
    uefi_system_table: 152,
    rsdp: 160,
    device_tree: 168,
    framebuffer: 184,
    command_line: 216,
    initial_ramdisk: 232,
    serial_console: 248,
});
layout!(RawRange, 16, { base: 0, size: 8 });
layout!(RawTable, 24, { base: 0, count: 8, entry_size: 16 });
layout!(RawMemoryRegion, 24, { base: 0, size: 8, kind: 16, reserved: 20 });
layout!(RawFramebuffer, 32, {
    base: 0,
    size: 8,
    width: 16,
    height: 20,
    stride: 24,
    pixel_format: 28,
});
layout!(RawSerialConsole, 16, { base: 0, kind: 8, reserved: 12 });
