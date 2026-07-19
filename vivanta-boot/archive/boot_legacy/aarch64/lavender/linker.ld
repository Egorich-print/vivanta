SECTIONS
{
    . = 0x40000000;
    __kernel_start = .;

    .text : {
        KEEP(*(.text._start))
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data*)
    }

    .bss : {
        __bss_start = .;
        *(.bss*)
        *(COMMON)
        __bss_end = .;
    }

    . = ALIGN(16);
    . += 16384;
    __stack_top = .;

    /DISCARD/ : {
        *(.comment*)
        *(.note*)
        *(.eh_frame*)
        *(.eh_frame_hdr*)
        *(.rela*)
        *(.debug*)
    }
}
