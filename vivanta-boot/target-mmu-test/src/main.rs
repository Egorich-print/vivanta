// Minimal MMU smoke test — no allocator, no Rust runtime.
// Creates static page tables, identity-maps RAM + UART, enables MMU.
// If this works, the MMU configuration is correct.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

// ── UART ────────────────────────────────────────────────────────────────────
const UART_BASE: u64 = 0x0900_0000;

fn uart_write(byte: u8) {
    unsafe {
        let uart = UART_BASE as *mut u32;
        // Wait for TX FIFO not full
        while core::ptr::read_volatile(uart.add(0x18 / 4)) & (1 << 5) != 0 {}
        core::ptr::write_volatile(uart, byte as u32);
    }
}

fn uart_print(s: &str) {
    for &b in s.as_bytes() {
        uart_write(b);
    }
}

// ── Page tables (static, 4KiB-aligned) ──────────────────────────────────────

#[repr(C, align(4096))]
struct PageTable([u64; 512]);

// L1 table: covers 0x0000_0000 – 0x7FFF_FFFF (512 × 1GB)
static mut L1_TABLE: PageTable = PageTable([0; 512]);

// L2 table for L1[0]: covers 0x0000_0000 – 0x3FFF_FFFF (512 × 2MB)
// UART at 0x0900_0000 is in L2[72]
static mut L2_TABLE_0: PageTable = PageTable([0; 512]);

// L2 table for L1[1]: covers 0x4000_0000 – 0x7FFF_FFFF (512 × 2MB)
// RAM at 0x4000_0000 – 0x47FF_FFFF uses L2[0..63]
static mut L2_TABLE_1: PageTable = PageTable([0; 512]);

// ── Descriptor helpers ──────────────────────────────────────────────────────

/// L2 block descriptor: 2MB block, identity-mapped.
fn block_desc(pa: u64, device: bool) -> u64 {
    let mut d = pa & 0x0000_FFFF_FFE0_0000; // PA bits [47:21]
    d |= 1 << 10; // AF (Access Flag)
    d |= 1 << 0; // Valid
    if device {
        // Device memory: AttrIdx=1, SH=Non-shareable
        d |= 1 << 2; // AttrIdx = 1
    } else {
        // Normal memory: AttrIdx=0, SH=Inner Shareable
        d |= 3 << 8; // SH = Inner Shareable
    }
    d
}

/// L1 table descriptor: points to L2 table.
fn table_desc(pa: u64) -> u64 {
    (pa & 0x0000_FFFF_FFFF_F000) | (1 << 1) | (1 << 0)
}

// ── Entry point ─────────────────────────────────────────────────────────────

// Use global_asm! to ensure _start is in .text._start section
// Stack is defined in linker.ld as __stack_top
core::arch::global_asm!(
    ".section .text._start, \"ax\"",
    ".global _start",
    ".type _start, %function",
    "_start:",
    "adrp x0, __stack_top",
    "add  x0, x0, :lo12:__stack_top",
    "mov sp, x0",
    "bl {rust_main}",
    "1: wfi",
    "b 1b",
    rust_main = sym rust_main,
);

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    // 1. Zero page tables
    unsafe {
        core::ptr::write_bytes(&raw mut L1_TABLE as *mut u8, 0, 4096);
        core::ptr::write_bytes(&raw mut L2_TABLE_0 as *mut u8, 0, 4096);
        core::ptr::write_bytes(&raw mut L2_TABLE_1 as *mut u8, 0, 4096);
    }

    // 2. Build L1 table
    //    L1[0] → L2_TABLE_0 (covers 0x0000_0000 – 0x3FFF_FFFF)
    //    L1[1] → L2_TABLE_1 (covers 0x4000_0000 – 0x7FFF_FFFF)
    unsafe {
        L1_TABLE.0[0] = table_desc(&raw const L2_TABLE_0 as u64);
        L1_TABLE.0[1] = table_desc(&raw const L2_TABLE_1 as u64);
    }

    // 3. Build L2_TABLE_0: map UART at 0x0900_0000 as 2MB Device block
    //    L2[72] = 0x0900_0000 (Device, 2MB block)
    unsafe {
        L2_TABLE_0.0[72] = block_desc(0x0900_0000, true);
    }

    // 4. Build L2_TABLE_1: map RAM at 0x4000_0000 – 0x47FF_FFFF as 2MB Normal blocks
    //    L2[0] = 0x4000_0000, L2[1] = 0x4020_0000, ..., L2[63] = 0x47E0_0000
    for i in 0..64 {
        let pa = 0x4000_0000u64 + (i as u64 * 0x20_0000);
        unsafe {
            L2_TABLE_1.0[i] = block_desc(pa, false);
        }
    }

    // 5. DSB to ensure page table writes are visible
    unsafe {
        core::arch::asm!("dsb sy");
    }

    // 6. Configure MAIR_EL1
    //    Attr0 = 0xFF (Normal, Inner/Outer WB, RA, WA)
    //    Attr1 = 0x04 (Device, nGnRnE)
    unsafe {
        core::arch::asm!("msr mair_el1, {}", in(reg) 0x0000_0000_0000_04FFu64);
    }

    // 7. Configure TCR_EL1
    //    T0SZ = 25 (VA range = 2^(64-25) = 2^39 = 512GB)
    //    SH0 = 0b11 (Inner Shareable)
    //    ORGN0 = 0b01 (Write-Back, Write-Allocate)
    //    IRGN0 = 0b01 (Write-Back, Write-Allocate)
    //    EPD1 = 1 (disable TTBR1 walks)
    //    IPS = 0b010 (40-bit PA)
    let tcr: u64 = (25)           // T0SZ
        | (0b11 << 8)             // SH0
        | (0b01 << 10)            // ORGN0
        | (0b01 << 12)            // IRGN0
        | (1 << 23)               // EPD1
        | (0b010 << 32); // IPS
    unsafe {
        core::arch::asm!("msr tcr_el1, {}", in(reg) tcr);
    }

    // 8. Set TTBR0_EL1 to L1 table
    let l1_addr = &raw const L1_TABLE as u64;
    unsafe {
        core::arch::asm!("msr ttbr0_el1, {}", in(reg) l1_addr);
    }

    // 9. DSB + ISB before enabling MMU
    unsafe {
        core::arch::asm!("dsb sy");
        core::arch::asm!("isb");
    }

    // 10. Enable MMU (SCTLR_EL1.M=1, C=1, I=1)
    let mut sctlr: u64;
    unsafe {
        core::arch::asm!("mrs {}, sctlr_el1", out(reg) sctlr);
    }
    sctlr |= (1 << 0) | (1 << 2) | (1 << 12);
    unsafe {
        core::arch::asm!("msr sctlr_el1, {}", in(reg) sctlr);
        core::arch::asm!("dsb sy");
        core::arch::asm!("isb");
    }

    // 11. UART write — this is the critical test!
    //     If MMU is working, this should succeed because UART is identity-mapped.
    uart_print("MMU-OK\n");

    // 12. Halt
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
