// ---------------------------------------------------------------------------
// arch-api — Vivanta ISA Contract
//
// Architecture-independent extern "Rust" declarations and opaque types
// that the vivanta_kernel uses to interact with ISA implementations.
// No traits, no C ABI, no concrete architecture types.
// ---------------------------------------------------------------------------

#![no_std]

/// Boot-time init functions — called by kernel_main
pub mod boot;

/// Thread context: opaque handles for context switching
pub mod context;

/// Scheduler callbacks: vivanta_kernel-provided functions called from arch
pub mod scheduler;

/// Page mapping flags: opaque bitmask for MMIO/RAM mapping
pub mod mmu;

/// Physical frame allocator contract
pub mod pmm;

/// User memory validation contract
pub mod user_memory;

/// Interrupt abstraction: RAII guard for disabled interrupts
pub mod interrupts;

/// Syscall dispatch: vivanta_kernel-provided function called from arch
pub mod syscall;
