ENTRY(_start)
KERNEL_IMAGE_BASE = 0xffffffff80000000;

PHDRS {
    text   PT_LOAD FLAGS(5);
    rodata PT_LOAD FLAGS(4);
    data   PT_LOAD FLAGS(6);
}

SECTIONS {
    . = KERNEL_IMAGE_BASE;
    .text   : ALIGN(4K) { *(.text .text.*) } :text
    .rodata : ALIGN(4K) { *(.rodata .rodata.*) } :rodata
    .data   : ALIGN(4K) { *(.data .data.*) } :data
    .bss    : ALIGN(4K) { *(.bss .bss.*) *(COMMON) } :data
    /DISCARD/ : { *(.eh_frame) *(.eh_frame_hdr) *(.note.*) *(.comment) }
}
