.section .text._start, "ax"
.global _start
.type _start, %function
_start:
    ldr r0, stack_top_ptr
    add sp, r0, #0
    b rust_main
stack_top_ptr:
    .word __stack_top
