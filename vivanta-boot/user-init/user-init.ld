ENTRY(_start)
SECTIONS
{
    . = 0x01000000;
    .text : {
        *(.text .text.*)
    }
    .rodata : {
        *(.rodata .rodata.*)
    }
    /DISCARD/ : {
        *(.comment) *(.note*) *(.eh_frame*)
    }
}
