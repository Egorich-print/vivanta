// ---------------------------------------------------------------------------
// PanicHandler — minimal page fault handler (ADR-011)
//
// PageFaultHandler trait and FaultResolution removed (Stage 5 deferred).
// ---------------------------------------------------------------------------

/// Panic on any page fault with register dump.
pub fn handle_page_fault(virt_addr: u64, write: bool, user: bool, instruction: bool) -> ! {
    panic!(
        "Unhandled page fault: virt=0x{:x} write={} user={} instr={}",
        virt_addr, write, user, instruction
    )
}
