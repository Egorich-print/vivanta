// ---------------------------------------------------------------------------
// M0.9 Exception Foundation — vectors, exception handler, ESR decoder
// Minimum viable EL2 exception infrastructure for target-rk3568
// ---------------------------------------------------------------------------

use core::arch::asm;
use vivanta_boot_common::println;

/// Saved CPU state at exception entry (matches asm save_and_halt macro layout).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExceptionFrame {
    pub x: [u64; 31],   // x0–x30 (x30 = LR)
    pub sp: u64,        // SP before exception
    pub elr: u64,       // ELR_EL2
    pub spsr: u64,      // SPSR_EL2
}

const EXCEPTION_NAMES: [&str; 16] = [
    "EL2t Sync",  "EL2t IRQ",  "EL2t FIQ",  "EL2t SError",
    "EL2h Sync",  "EL2h IRQ",  "EL2h FIQ",  "EL2h SError",
    "AArch64 Lo", "AArch64 Lo", "AArch64 Lo", "AArch64 Lo",
    "AArch32 Lo", "AArch32 Lo", "AArch32 Lo", "AArch32 Lo",
];

fn esr_class(esr: u64) -> &'static str {
    match (esr >> 26) & 0x3f {
        0b000000 => "Unknown",
        0b000001 => "Trapped WFI/WFE",
        0b000011 => "Trapped MCR/MRC",
        0b000100 => "Trapped MCRR/MRRC",
        0b000101 => "Trapped MCR/MRC (AArch32)",
        0b000110 => "Trapped LDC/STC",
        0b000111 => "SVE/SIMD/FP",
        0b001010 => "Trapped MSR/MRS (system)",
        0b001100 => "Trapped LD64B/ST64B",
        0b001011 => "Trapped PAC",
        0b010001 => "SVC (AArch64)",
        0b010010 => "HVC (AArch64)",
        0b010011 => "SMC (AArch64)",
        0b010100 => "SVC (AArch32)",
        0b010101 => "HVC (AArch32)",
        0b010110 => "SMC (AArch32)",
        0b011000 => "Trapped MSR/MRS (AArch32)",
        0b011100 => "Pointer Auth (AArch64)",
        0b100000 => "Instruction Abort (lower EL)",
        0b100001 => "Instruction Abort (same EL)",
        0b100100 => "Data Abort (lower EL)",
        0b100101 => "Data Abort (same EL)",
        0b101000 => "SP Alignment fault",
        0b101100 => "FP/SIMD trap (AArch64)",
        0b101111 => "SError",
        0b110000 => "Breakpoint (lower EL)",
        0b110001 => "Breakpoint (same EL)",
        0b110010 => "SW Step (lower EL)",
        0b110011 => "SW Step (same EL)",
        0b110100 => "Watchpoint (lower EL)",
        0b110101 => "Watchpoint (same EL)",
        0b111000 => "BKPT (AArch32)",
        0b111100 => "BRK (AArch64)",
        _ => "Reserved",
    }
}

/// Called from the assembly vector table.
#[no_mangle]
pub unsafe extern "C" fn exception_handler(
    frame: &ExceptionFrame,
    kind: u64,
    esr: u64,
    far: u64,
) -> ! {
    let name = if (kind as usize) < EXCEPTION_NAMES.len() {
        EXCEPTION_NAMES[kind as usize]
    } else {
        "Unknown"
    };
    let class = esr_class(esr);
    let ec = (esr >> 26) & 0x3f;
    let il = (esr >> 25) & 1;
    let iss = esr & 0x1FF_FFFF;

    println!();
    println!("+===============================================+");
    println!("|  VIVANTA EXCEPTION");
    println!("+-----------------------------------------------+");
    println!("|  Vector:  {:<2} — {}", kind, name);
    println!("|  Class:   {} (EC=0x{:02x})", class, ec);
    println!("|  IL:      {}", il);
    println!("|  ISS:     0x{:07x}", iss);
    println!("+-----------------------------------------------+");
    println!("|  ESR_EL2:  {:#018x}", esr);
    println!("|  FAR_EL2:  {:#018x}", far);
    println!("|  ELR_EL2:  {:#018x}", frame.elr);
    println!("|  SPSR_EL2: {:#018x}", frame.spsr);
    println!("|  SP:       {:#018x}", frame.sp);
    println!("+-----------------------------------------------+");
    println!("|  x0:  {:#018x}  x1:  {:#018x}", frame.x[0], frame.x[1]);
    println!("|  x2:  {:#018x}  x3:  {:#018x}", frame.x[2], frame.x[3]);
    println!("|  x4:  {:#018x}  x5:  {:#018x}", frame.x[4], frame.x[5]);
    println!("|  x6:  {:#018x}  x7:  {:#018x}", frame.x[6], frame.x[7]);
    println!("|  x8:  {:#018x}  x9:  {:#018x}", frame.x[8], frame.x[9]);
    println!("|  x10: {:#018x}  x11: {:#018x}", frame.x[10], frame.x[11]);
    println!("|  x12: {:#018x}  x13: {:#018x}", frame.x[12], frame.x[13]);
    println!("|  x14: {:#018x}  x15: {:#018x}", frame.x[14], frame.x[15]);
    println!("|  x16: {:#018x}  x17: {:#018x}", frame.x[16], frame.x[17]);
    println!("|  x18: {:#018x}  x19: {:#018x}", frame.x[18], frame.x[19]);
    println!("|  x20: {:#018x}  x21: {:#018x}", frame.x[20], frame.x[21]);
    println!("|  x22: {:#018x}  x23: {:#018x}", frame.x[22], frame.x[23]);
    println!("|  x24: {:#018x}  x25: {:#018x}", frame.x[24], frame.x[25]);
    println!("|  x26: {:#018x}  x27: {:#018x}", frame.x[26], frame.x[27]);
    println!("|  x28: {:#018x}  x29: {:#018x}", frame.x[28], frame.x[29]);
    println!("|  x30: {:#018x}", frame.x[30]);
    println!("+===============================================+");
    println!("  CPU halted");
    println!();

    loop {
        core::hint::spin_loop();
    }
}

/// Install EL2 exception vectors from the asm table.
pub unsafe fn init() {
    let v: u64;
    asm!(
        "adrp {v}, exception_vectors",
        "add {v}, {v}, :lo12:exception_vectors",
        "msr VBAR_EL2, {v}",
        "isb",
        v = out(reg) v,
    );
    println!("[exn] VBAR_EL2 = 0x{:x}", v);
}

/// Test: trigger synchronous exception via BRK #0.
pub unsafe fn test_brk() {
    println!("[exn] triggering BRK #0 …");
    asm!("brk #0");
}

/// Test: trigger Data Abort via invalid memory access.
#[allow(dead_code)]
pub unsafe fn test_fault() {
    println!("[exn] triggering Data Abort via null ptr read …");
    core::ptr::read_volatile(core::ptr::null::<u64>());
}
