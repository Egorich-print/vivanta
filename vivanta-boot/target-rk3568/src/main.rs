#![no_std]
#![no_main]

mod exceptions;

use core::arch::asm;
use core::panic::PanicInfo;

use vivanta_boot_common::{MemoryMap, fdt::FdtScanner, ns16550::Ns16550, println, set_console};

// ================================================================
// ARM64 Image header + stack setup
// ================================================================
core::arch::global_asm!(
    ".section .text._start",
    ".global _start",
    "_start:",
    "b _real_start",
    ".word 0",
    ".quad 0",
    ".quad __kernel_end - _start",
    ".quad 0",
    ".quad 0x20500000",
    ".quad 0",
    ".quad 0",
    ".word 0x644d5241",
    ".word 0",
    "_real_start:",
    "msr daifset, #0xf",
    "adrp x1, __stack_top",
    "add x1, x1, :lo12:__stack_top",
    "mov sp, x1",
    "b boot_entry",
);

// ================================================================
// EL2 exception vector table — 16 entries × 128 bytes = 2048 bytes
// Adapted from arch-aarch64: EL1 registers → EL2 registers
// ================================================================
core::arch::global_asm!(
    ".section .text.vectors, \"ax\"",
    ".balign 2048",
    ".global exception_vectors",
    "exception_vectors:",
    // === EL2t (SP_EL0) ===
    ".balign 128; b   el2t_sync",
    ".balign 128; b   el2t_irq",
    ".balign 128; b   el2t_fiq",
    ".balign 128; b   el2t_serror",
    // === EL2h (SP_EL2) — current execution mode ===
    ".balign 128; b   el2h_sync",
    ".balign 128; b   el2h_irq",
    ".balign 128; b   el2h_fiq",
    ".balign 128; b   el2h_serror",
    // === Lower EL, AArch64 ===
    ".balign 128; b   lower64_sync",
    ".balign 128; b   lower64_irq",
    ".balign 128; b   lower64_fiq",
    ".balign 128; b   lower64_serror",
    // === Lower EL, AArch32 ===
    ".balign 128; b   lower32_sync",
    ".balign 128; b   lower32_irq",
    ".balign 128; b   lower32_fiq",
    ".balign 128; b   lower32_serror",
    // ============================================================
    // save_and_halt — save full CPU context, call exception_handler, halt
    // ============================================================
    ".macro save_and_halt kind",
    "    sub   sp, sp, #(34 * 8)",
    "    stp   x0, x1,  [sp, #(0  * 8)]",
    "    stp   x2, x3,  [sp, #(2  * 8)]",
    "    stp   x4, x5,  [sp, #(4  * 8)]",
    "    stp   x6, x7,  [sp, #(6  * 8)]",
    "    stp   x8, x9,  [sp, #(8  * 8)]",
    "    stp   x10,x11, [sp, #(10 * 8)]",
    "    stp   x12,x13, [sp, #(12 * 8)]",
    "    stp   x14,x15, [sp, #(14 * 8)]",
    "    stp   x16,x17, [sp, #(16 * 8)]",
    "    stp   x18,x19, [sp, #(18 * 8)]",
    "    stp   x20,x21, [sp, #(20 * 8)]",
    "    stp   x22,x23, [sp, #(22 * 8)]",
    "    stp   x24,x25, [sp, #(24 * 8)]",
    "    stp   x26,x27, [sp, #(26 * 8)]",
    "    stp   x28,x29, [sp, #(28 * 8)]",
    "    str   x30,     [sp, #(30 * 8)]",
    "    add   x0, sp, #(34 * 8)",
    "    str   x0,      [sp, #(31 * 8)]", // saved SP
    "    mrs   x1, elr_el2",
    "    str   x1,      [sp, #(32 * 8)]", // ELR_EL2
    "    mrs   x2, spsr_el2",
    "    str   x2,      [sp, #(33 * 8)]", // SPSR_EL2
    "    mov   x0, sp",
    "    mov   x1, \\kind",
    "    mrs   x2, esr_el2",
    "    mrs   x3, far_el2",
    "    bl    exception_handler",
    "    b     .",
    ".endm",
    // === Vector dispatch — all go to halt for M0.9 ===
    "el2t_sync:        save_and_halt 0",
    "el2t_irq:         save_and_halt 1",
    "el2t_fiq:         save_and_halt 2",
    "el2t_serror:      save_and_halt 3",
    "el2h_sync:        save_and_halt 4",
    "el2h_irq:         save_and_halt 5",
    "el2h_fiq:         save_and_halt 6",
    "el2h_serror:      save_and_halt 7",
    "lower64_sync:     save_and_halt 8",
    "lower64_irq:      save_and_halt 9",
    "lower64_fiq:      save_and_halt 10",
    "lower64_serror:   save_and_halt 11",
    "lower32_sync:     save_and_halt 12",
    "lower32_irq:      save_and_halt 13",
    "lower32_fiq:      save_and_halt 14",
    "lower32_serror:   save_and_halt 15",
);

// ================================================================
// Rust entry point
// ================================================================
#[unsafe(no_mangle)]
pub unsafe extern "C" fn boot_entry(dtb: u64) -> ! {
    unsafe {
        disable_mmu_el2();
        zero_bss();

        // Console
        static NS16550: Ns16550 = Ns16550::new(0xFE66_0000 as *mut u8, 2);
        set_console(&NS16550);

        println!();
        println!("=== Vivanta RK3568 (EL2) ===");

        // Install exception vectors
        exceptions::init();

        // DTB
        let dtb_ptr = if dtb > 0x100 && dtb < 0x1_0000_0000 {
            dtb as *const u8
        } else {
            core::ptr::null()
        };

        if !dtb_ptr.is_null() {
            let mut mem = MemoryMap::new();
            {
                let cpus = FdtScanner::report(dtb_ptr, &mut mem);
                println!("CPU cores: {}", cpus);
                println!("Memory:");
                let mut usable = 0u64;
                for r in mem.regions() {
                    if r.start == 0 && r.size == 0 {
                        continue;
                    }
                    usable += 1;
                    println!(
                        "  {}. 0x{:016X} – 0x{:016X} ({} MiB)",
                        usable,
                        r.start,
                        r.start + r.size - 1,
                        r.size >> 20,
                    );
                }
                println!("  total: {} regions", usable);
            }
        } else {
            println!("DTB: not provided");
        }

        // === M0.9 Tests ===
        println!();
        println!("--- Exception Tests ---");
        exceptions::test_brk();

        loop {}
    }
}

// ================================================================
// Early init
// ================================================================

unsafe fn disable_mmu_el2() {
    unsafe {
        asm!("dsb sy; isb");
        let sctlr: u64;
        asm!("mrs {}, sctlr_el2", out(reg) sctlr);
        asm!("msr sctlr_el2, {}", in(reg) sctlr & !0b101u64);
        asm!("dsb sy; isb; tlbi alle2; dsb sy; isb");
        asm!("msr CPTR_EL2, xzr");
        asm!("msr CPACR_EL1, {}", in(reg) (0b11u64 << 20));
    }
}

unsafe fn zero_bss() {
    unsafe extern "C" {
        static mut __bss_start: u8;
        static mut __bss_end: u8;
    }
    let start = core::ptr::addr_of_mut!(__bss_start) as *mut u64;
    let end = core::ptr::addr_of_mut!(__bss_end) as *mut u64;
    let count = (end as usize - start as usize) / 8;
    for i in 0..count {
        unsafe { start.add(i).write_volatile(0) };
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
