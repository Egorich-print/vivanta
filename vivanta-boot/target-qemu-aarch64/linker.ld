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
    /* 256 KiB: kernel_main runs every boot gate inline and holds
     * ~10 KiB AddressSpace/VaAllocator temporaries by value; the -O0
     * frame peaked past 128 KiB once the P0 gates landed, overflowing
     * into .bss (silent console death pre-dump). Gates are moving to
     * #[inline(never)] fns, but the stack keeps 2x headroom regardless.
     * (M6: 16->32, mission-4: 32->64, G-M9: 64->128, P0: 128->256.) */
    . += 262144;
    __stack_top = .;

    /DISCARD/ : {
        *(.comment .note.* .eh_frame .rela.*)
    }
}
