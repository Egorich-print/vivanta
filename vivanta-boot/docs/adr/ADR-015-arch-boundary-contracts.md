> **Note:** At the time of writing, the project was named TheseusOS. The content reflects the historical state and is preserved as-is.
>
# ADR-015: Arch Boundary Contracts

**Status:** Accepted  
**Date:** 2026-07-14  
**Related:** ADR-014 (Architectural Boundaries)

---

## Context

ADR-014 established the dependency direction rules:
kernel→arch-api, arch→arch-api, but NOT kernel→arch-aarch64.

This ADR defines **how** the layers communicate across that boundary.

The question: how does the kernel call architecture-specific functions
without a direct crate dependency and without runtime polymorphism
(traits with vtables)?

## Decision

### 1. Mechanism: `extern "Rust"` blocks

`arch-api` declares function signatures in `extern "Rust"` blocks.
Each arch implementation provides `#[no_mangle]` functions.

```rust
// arch-api/src/context.rs
pub mod context {
    extern "Rust" {
        pub fn switch_context(old: &mut usize, new: usize);
    }
}
```

```rust
// arch-aarch64/src/context.rs
#[no_mangle]
pub fn switch_context(old: &mut usize, new: usize) {
    // AArch64 implementation
}
```

- Kernel calls `arch_api::context::switch_context(&mut old, new)`.
- Linker resolves to the `#[no_mangle]` `switch_context` symbol.
- Only ONE arch implementation crate is linked per target binary.
- No vtable, no runtime dispatch, no trait boilerplate.

### 2. Why not `extern "C"`?

The existing codebase uses `extern "C"` for exception handlers. However:

- `extern "C"` fixes the ABI as the platform C calling convention.
- `extern "Rust"` uses the Rust calling convention, which allows the
  compiler to optimize across the boundary (with LTO).
- Since all crates are compiled by the same rustc, Rust ABI is stable
  within the build.
- If a C boundary is needed later (linker scripts, bootloaders), it can
  be added selectively. Not needed now.

### 3. Why not `trait Architecture`?

A trait forces creating a "least common denominator" for all ISAs before
a second implementation exists. This violates "no abstraction before
second implementation."

A `trait Arch` would require:
- `type PageTable`, `type Frame`, `type ExceptionFrame`, etc.
- Virtual dispatch or monomorphization for every function.
- Generic bounds on every kernel function.

`extern "Rust"` functions achieve the same build-time selection with
zero trait infrastructure. Abstraction can be added later when real
divergence demands it.

### 4. Bidirectional Communication

Some information flows FROM arch TO kernel:

- Timer interrupt handler (in arch) calls `maybe_reschedule()` (in kernel).
- Exception handler (in arch) may call kernel panic.

This is handled by declaring kernel-provided callbacks in `arch-api`:

```rust
// arch-api/src/scheduler.rs
extern "Rust" {
    pub fn maybe_reschedule(frame_ptr: usize);
}
```

```rust
// kernel/src/scheduler.rs
#[no_mangle]
pub fn maybe_reschedule(frame_ptr: usize) {
    // kernel scheduler logic
}
```

- Arch crate declares it extern (no dependency on kernel crate).
- Kernel crate provides the `#[no_mangle]` definition.
- Target links both, linker resolves the symbol.
- **No circular crate dependency.**

### 5. Thread `arch_state` Token

The kernel's `Thread` struct stores an opaque architecture token:

```rust
pub struct Thread {
    pub id: ThreadId,
    pub state: ThreadState,
    pub stack: Stack,
    pub arch_state: usize,  // opaque SP / context token
}
```

`arch_state` semantics (defined by the arch implementation, not kernel):
- On AArch64: `arch_state` = saved stack pointer for this thread.
  Callee-saved registers (x19–x30) are stored on the thread's stack.
  The `ExceptionFrame` (x0–x30, sp, elr, spsr) is stored on the
  exception stack, referenced by the SP.

- Kernel never reads/writes `arch_state` directly — it passes it to
  `arch_api::switch_context(&mut old, new)`.

### 6. MappingFlags

`MappingFlags` lives in `arch-api`, not `boot-info`:

- `boot-info` describes **what hardware exists** (MmioRegion, InterruptControllerInfo).
- `arch-api` describes **how the kernel requests mappings** (MappingFlags).

```rust
// arch-api/src/mmu.rs
pub struct MappingFlags { bits: u64 }

impl MappingFlags {
    pub fn read_write() -> Self;
    pub fn execute() -> Self;
    pub fn user() -> Self;
    pub fn device() -> Self;
}
```

Kernel calls `arch_api::map_range(pt, vaddr, paddr, size, flags)` where
`flags` is constructed via these methods. Kernel does not know the bit
values.

### 7. BootInfo Location

`BootInfo` lives in a separate crate `boot-info`, depending only on
`core`. It contains:

```rust
#[repr(C)]
pub struct BootInfo {
    pub memory_map: MemoryMap,
    pub mmio_segments: &'static [MmioRegion],
    pub interrupt_controller: Option<InterruptControllerInfo>,
    pub cpu_count: usize,
    pub dtb: Option<usize>,
}
```

- No `arch` field (diagnostic, not a contract).
- No `boot_source` field (diagnostic).
- No `acpi`, `framebuffer`, `cmdline`, `initrd` (premature).

`boot_common` re-exports from `boot-info` for convenience and adds
runtime utilities (Console, println!, FDT parser, ns16550 driver).

### 8. No `platform-api`, No `device-hal`, No `driver-framework`

Until a second implementation of any boundary exists (per ADR-014
principle), we do NOT create:
- `pub trait Uart`
- `pub trait InterruptController`
- `pub trait DeviceDriver`
- `pub struct Architecture`

These are rejected. Any need for them will be evaluated per-case when
actual divergence requires it.

## Consequences

- `arch-api` contains `extern "Rust"` declarations and `MappingFlags`.
  No concrete types (ExceptionFrame, PageFlags, etc.).
- `arch-aarch64` provides `#[no_mangle]` implementations for every
  `extern "Rust"` function in arch-api.
- `arch-aarch64::ExceptionFrame` becomes private to the arch crate.
  Kernel only sees `usize`.
- `boot-info` crate has zero dependencies besides `core`.
- Timer handler in arch calls `arch_api::maybe_reschedule()` which is
  resolved to a kernel-provided `#[no_mangle]` function.
- `cargo tree -p kernel` will show `kernel → arch-api, boot-info`
  — never `kernel → arch-aarch64`.