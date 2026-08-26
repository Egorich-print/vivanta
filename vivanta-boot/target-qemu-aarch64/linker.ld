PHDRS
{
    text PT_LOAD FLAGS(5);
    data PT_LOAD FLAGS(6);
}

SECTIONS
{
    . = 0x40200000;
    __kernel_start = .;

    .text : ALIGN(16) {
        KEEP(*(.text._start))
        *(.text .text.*)
        *(.user.text)
    } :text

    .rodata : ALIGN(16) {
        *(.rodata .rodata.*)
    } :text

    .data : ALIGN(16) {
        *(.data .data.*)
    } :data

    .bss : ALIGN(16) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } :data

    . = ALIGN(16);
    __stack_bottom = .;
    /* 128 KiB: kernel_main locals + large AddressSpace temporaries
     * (each struct evolution raises the peak; M6: 16->32, mission-4:
     * 32->64, G-M9: 64->128 as Mapping gained COW metadata). */
    . += 131072;
    __stack_top = .;

    /DISCARD/ : {
        *(.comment .note.* .eh_frame .rela.*)
    }
}
