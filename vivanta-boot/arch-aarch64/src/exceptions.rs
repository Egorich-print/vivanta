// ---------------------------------------------------------------------------
// AArch64 exception handling
// ---------------------------------------------------------------------------

use vivanta_boot_common::println;

/// Saved CPU state at exception entry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExceptionFrame {
    pub x: [u64; 31],
    pub sp: u64,
    pub elr: u64,
    pub spsr: u64,
}

const EXCEPTION_NAMES: [&str; 16] = [
    "EL1t Sync",
    "EL1t IRQ",
    "EL1t FIQ",
    "EL1t SError",
    "EL1h Sync",
    "EL1h IRQ",
    "EL1h FIQ",
    "EL1h SError",
    "Lower EL AArch64 Sync",
    "Lower EL AArch64 IRQ",
    "Lower EL AArch64 FIQ",
    "Lower EL AArch64 SError",
    "Lower EL AArch32 Sync",
    "Lower EL AArch32 IRQ",
    "Lower EL AArch32 FIQ",
    "Lower EL AArch32 SError",
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
        0b001010 => "Trapped MSR/MRS",
        0b001100 => "Trapped LD64B/ST64B",
        0b001011 => "Trapped PAC",
        0b010000 => "SVC (AArch32)",
        0b010001 => "HVC (AArch32)",
        0b010010 => "SMC (AArch32)",
        0b010101 => "SVC (AArch64)",
        0b010110 => "HVC (AArch64)",
        0b011000 => "SMC (AArch64)",
        0b011001 => "Trapped IMPLEMENTATION DEFINED",
        0b011100 => "Pointer Auth (AArch64)",
        0b100000 => "Instruction Abort (lower EL)",
        0b100001 => "Instruction Abort (same EL)",
        0b100010 => "Instruction Abort not used",
        0b100100 => "Data Abort (lower EL)",
        0b100101 => "Data Abort (same EL)",
        0b100110 => "Data Abort not used",
        0b101000 => "SP Alignment",
        0b101100 => "FP/SIMD (AArch64)",
        0b101111 => "SError",
        0b110000 => "Breakpoint (lower EL)",
        0b110001 => "Breakpoint (same EL)",
        0b110010 => "SW Step (lower EL)",
        0b110011 => "SW Step (same EL)",
        0b110100 => "Watchpoint (lower EL)",
        0b110101 => "Watchpoint (same EL)",
        0b111000 => "BKPT (AArch32)",
        0b111100 => "Vector Catch (AArch32)",
        _ => "Reserved",
    }
}

/// Called from the assembly vector table.
#[unsafe(no_mangle)]
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

    println!();
    println!("{:=^48}", "");
    println!("  VIVANTA EXCEPTION");
    println!("{:=^48}", "");
    println!();
    println!("  Vector:    {} ({})", name, kind);
    println!(
        "  Class:     {} (ESR[31:26] = {:#x})",
        class,
        (esr >> 26) & 0x3f
    );
    println!();
    println!("  ESR_EL1:   {:#018x}", esr);
    println!("  FAR_EL1:   {:#018x}", far);
    println!("  ELR_EL1:   {:#018x}", frame.elr);
    println!("  SPSR_EL1:  {:#018x}", frame.spsr);
    println!("  SP:        {:#018x}", frame.sp);
    println!();
    println!("  x0:  {:#018x}   x1:  {:#018x}", frame.x[0], frame.x[1]);
    println!("  x2:  {:#018x}   x3:  {:#018x}", frame.x[2], frame.x[3]);
    println!("  x4:  {:#018x}   x5:  {:#018x}", frame.x[4], frame.x[5]);
    println!("  x6:  {:#018x}   x7:  {:#018x}", frame.x[6], frame.x[7]);
    println!("  x8:  {:#018x}   x9:  {:#018x}", frame.x[8], frame.x[9]);
    println!("  x10: {:#018x}   x11: {:#018x}", frame.x[10], frame.x[11]);
    println!("  x12: {:#018x}   x13: {:#018x}", frame.x[12], frame.x[13]);
    println!("  x14: {:#018x}   x15: {:#018x}", frame.x[14], frame.x[15]);
    println!("  x16: {:#018x}   x17: {:#018x}", frame.x[16], frame.x[17]);
    println!("  x18: {:#018x}   x19: {:#018x}", frame.x[18], frame.x[19]);
    println!("  x20: {:#018x}   x21: {:#018x}", frame.x[20], frame.x[21]);
    println!("  x22: {:#018x}   x23: {:#018x}", frame.x[22], frame.x[23]);
    println!("  x24: {:#018x}   x25: {:#018x}", frame.x[24], frame.x[25]);
    println!("  x26: {:#018x}   x27: {:#018x}", frame.x[26], frame.x[27]);
    println!("  x28: {:#018x}   x29: {:#018x}", frame.x[28], frame.x[29]);
    println!("  x30: {:#018x}", frame.x[30]);
    println!();
    println!("{:=^48}", "");
    println!("  CPU halted");
    println!("{:=^48}", "");
    println!();

    loop {
        core::hint::spin_loop();
    }
}

/// Deliberately trigger a Data Abort for regression testing.
pub unsafe fn trigger_fault() -> ! {
    unsafe {
        core::ptr::read_volatile(0x0 as *const u64);
        loop {
            core::hint::spin_loop();
        }
    }
}

pub fn init() {
    unsafe extern "C" {
        pub static exception_vectors: u8;
    }
    let vectors = &raw const exception_vectors as u64;
    unsafe {
        core::arch::asm!(
            "msr VBAR_EL1, {v}",
            "isb",
            v = in(reg) vectors,
        );
    }
    println!("  VBAR_EL1:  0x{:x}", vectors);
}
