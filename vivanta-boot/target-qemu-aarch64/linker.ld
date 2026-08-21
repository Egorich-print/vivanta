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
    /* 64 KiB: kernel_main locals + large AddressSpace temporaries
     * (VaAllocator grew the struct; M6 precedent: 16->32 KiB). */
    . += 65536;
    __stack_top = .;

    /DISCARD/ : {
        *(.comment .note.* .eh_frame .rela.*)
    }
}
