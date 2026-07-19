// ---------------------------------------------------------------------------
// Boot-time arch init — extern "Rust" declarations
//
// Kernel calls these during boot. Each arch implementation (arch-aarch64,
// arch-x86_64, ...) provides the #[no_mangle] implementations.
// ---------------------------------------------------------------------------

pub mod cpu;
pub mod mmu;
pub mod irq;
pub mod timer;
pub mod sched;
pub mod user;