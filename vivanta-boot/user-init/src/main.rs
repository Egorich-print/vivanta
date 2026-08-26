#![no_std]
#![no_main]

// Vivanta user-init — the first genuine ELF64 AArch64 user program (M8.3).
//
// Entirely naked assembly: no compiler-generated stack use, so the
// program can run before it has mapped a stack. The sequence is the
// same contract as the vmsys QEMU gate, proving the ELF loader +
// syscall ABI + demand-fill path work together:
//
//   mmap(8K, RW) → store/load (demand fill) → mprotect(RO) → munmap
//   → unknown=-ENOSYS, len=0=-EINVAL, W|X=-EPERM → exit(42)

core::arch::global_asm!(
    ".section .text",
    ".global _start",
    ".balign 4",
    "_start:",
    // mmap(addr=0, len=8192, prot=RW)
    "mov  x8, #4",
    "mov  x0, #0",
    "mov  x1, #8192",
    "mov  x2, #3",
    "svc  #0",
    "tbz  x0, 63, 1f",
    "mov  x0, #1",
    "b    .Lexit",
    "1:",
    "mov  x19, x0",
    // store + load through the new mapping
    "mov  x1, #0xDEAD",
    "str  x1, [x19]",
    "ldr  x2, [x19]",
    "cmp  x2, x1",
    "b.eq 2f",
    "mov  x0, #2",
    "b    .Lexit",
    "2:",
    // mprotect(base, 4096, RO)
    "mov  x8, #6",
    "mov  x0, x19",
    "mov  x1, #4096",
    "mov  x2, #1",
    "svc  #0",
    "cbz  x0, 3f",
    "mov  x0, #3",
    "b    .Lexit",
    "3:",
    // munmap(base, 8192)
    "mov  x8, #5",
    "mov  x0, x19",
    "mov  x1, #8192",
    "svc  #0",
    "cbz  x0, 4f",
    "mov  x0, #4",
    "b    .Lexit",
    "4:",
    // unknown syscall → -ENOSYS (-38)
    "mov  x8, #99",
    "svc  #0",
    "mov  x1, #-38",
    "cmp  x0, x1",
    "b.eq 5f",
    "mov  x0, #5",
    "b    .Lexit",
    "5:",
    // mmap len=0 → -EINVAL (-22)
    "mov  x8, #4",
    "mov  x0, #0",
    "mov  x1, #0",
    "mov  x2, #3",
    "svc  #0",
    "mov  x1, #-22",
    "cmp  x0, x1",
    "b.eq 6f",
    "mov  x0, #6",
    "b    .Lexit",
    "6:",
    // mmap prot=W|X → -EPERM (-1)
    "mov  x8, #4",
    "mov  x0, #0",
    "mov  x1, #4096",
    "mov  x2, #6",
    "svc  #0",
    "mov  x1, #-1",
    "cmp  x0, x1",
    "b.eq 7f",
    "mov  x0, #7",
    "b    .Lexit",
    "7:",
    "mov  x0, #42",
    ".Lexit:",
    "mov  x8, #2",
    "svc  #0",
    "b .",
);

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
