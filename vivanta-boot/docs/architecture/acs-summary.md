# Architecture Cleanup Sprint — Summary

**Date:** 2026-07-14
**Duration:** Single session
**Status:** Complete

## Goal

Separate ISA / SoC / Kernel / Target into independent crates with strict
dependency direction. Transition Vivanta from an ARM kernel to an
architecture-independent kernel with an ARM backend.

## Results

### Completed Phases

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | ADR-014 (Architectural Boundaries) | Done |
| 0.5 | ADR-015 (Arch Boundary Contracts) | Done |
| 1 | `boot-info` crate extraction | Done |
| 2 | MMIO addresses removed from kernel | Done |
| 3 | Kernel → arch-aarch64 dependency severed | Done |
| 4 | Scheduler split (policy in kernel, mechanism in arch) | Done |
| 5 | Arch API contract purification | Done |
| 6 | Dependency validation | Done |
| 7 | ~~Build-time proof with `arch-test-stub`~~ | **RETRACTED 2026-09-25** (see note) |

### Dependency Graph (Final)

```
kernel v0.1.0
├── arch-api v0.1.0
├── boot-common v0.1.0
│   └── boot-info v0.1.0
└── boot-info v0.1.0
```

No `arch-aarch64`, no `platform-*`, no `cfg(target_arch)`.

### Communication Mechanism

`extern "Rust"` declarations in `arch-api`. Arch implementations provide
`#[no_mangle]` functions. Linker resolves at build time. Bidirectional:
kernel → arch (MMU, context switch) and arch → kernel (timer tick,
scheduler reschedule).

### Key Architectural Changes

1. **BootInfo** — reduced from 9 fields to 5. Removed `arch`, `boot_source`,
   `acpi`, `framebuffer`, `cmdline`, `initrd`. Added `mmio_regions` and
   `interrupt_controller`.

2. **Thread** — replaced `ctx: ThreadContext` + `full: ExceptionFrame` with
   single `context: ArchContext` (opaque `usize`).

3. **arch-api** — removed `ExceptionFrame`, `PageFlags`, all HAL traits
   (`Mmu`, `InterruptController`, `Timer`, `ThreadManager`, etc.).
   Only `extern "Rust"` declarations, `MappingFlags`, and `FrameAllocator`
   remain.

4. **Scheduler** — `Thread`, `RunQueue`, policy moved to `kernel::scheduler`.
   `context_switch_asm`, `init_context` stay in `arch-aarch64::context`.

### Files Created/Modified

| Area | Files |
|------|-------|
| New crates | `boot-info/`, `arch-test-stub/`, `target-test/` |
| New modules | `arch-api/src/boot/`, `arch-api/src/context.rs`, `arch-api/src/scheduler.rs` |
| New modules | `arch-aarch64/src/boot.rs`, `arch-aarch64/src/context.rs` |
| New modules | `kernel/src/scheduler/` (thread.rs, runqueue.rs, mod.rs) |
| New ADRs | `docs/adr/ADR-014-architectural-boundaries.md`, `ADR-015-arch-boundary-contracts.md` |
| Modified | `kernel/Cargo.toml`, `kernel/src/lib.rs`, `kernel/src/vmm/mod.rs` |
| Modified | `arch-aarch64/src/mmu.rs`, `arch-aarch64/src/thread.rs`, `arch-aarch64/src/timer.rs` |
| Modified | `arch-aarch64/src/interrupts/dispatcher.rs`, `arch-aarch64/src/user.rs` |
| Modified | `arch-api/src/*`, `boot_common/src/lib.rs`, `boot_common/Cargo.toml` |
| Modified | `target-qemu-aarch64/src/main.rs`, `boot/aarch64/qemu_kernel/src/main.rs` |
| Modified | `Cargo.toml` (workspace), `build.sh` |

### Build-Time Proof — RETRACTED 2026-09-25

The `cargo build -p target-test` step claimed to link `kernel` against
`arch-test-stub` without a real architecture. It did not: `target-test` never
referenced `kernel_main`, so the kernel was never linked, and the stub was
missing 13 `arch-api` symbols the kernel actually calls. Both crates are
deleted. The architectural rule (kernel does not depend on a concrete ISA) is
enforced by dependency direction in `vivanta-boot/Cargo.toml` — `kernel`
depends on `arch-api` only, and each `target-*` picks the architecture.

### Pre-existing Issues (not resolved by this sprint)

- ~~`target-qemu-armv7a` — lifetime bug: `set_console(&uart)` requires `'static`~~ — crate deleted 2026-09-25 (unbuildable: not a workspace member, ARMv7 core not in the pinned toolchain)
- `target-qemu-aarch64` — EL0 bootstrap Data Abort at FAR `0x60`
- Several `#![warn(static_mut_refs)]` (safe in single-core context)
- `arch-armv7a` — frozen stub, not yet implemented

### Next Steps (Phase 8 — Platform Reality Layer)

- Real BootInfo pipeline (FDT-driven, not static regions)
- Remove `boot_common` leakage into kernel
- Platform validation matrix with real hardware results
- First real `arch-x86_64` with QEMU boot to `kernel_main()`