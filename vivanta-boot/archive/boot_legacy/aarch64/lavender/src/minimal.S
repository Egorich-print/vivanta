.section .text._start, "ax"
.global _start
_start:
    b _real_start
    .word 0
    .quad 0
    .quad 0x400000
    .quad 0x0a
    .quad 0
    .quad 0
    .quad 0
    .byte 0x41, 0x52, 0x4d, 0x64
    .word 0

.global _real_start
_real_start:
    /* wfi forever */
1:  wfi
    b 1b
