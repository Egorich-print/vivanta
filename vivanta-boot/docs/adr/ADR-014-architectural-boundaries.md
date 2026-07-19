> **Note:** At the time of writing, the project was named TheseusOS. The content reflects the historical state and is preserved as-is.
>
# ADR-014: Architectural Boundaries

**Status:** Accepted  
**Date:** 2026-07-14  
**Supersedes:** —  
**Related:** ADR-011 (Phase Transition), ADR-012 (Execution Model), ADR-013 (Privilege Transition)

---

## Context

Vivanta completed Stage 5 (preemptive scheduler) and Stage 6A (EL0 bootstrap
experiment) as a monolithic ARM-specific kernel. The kernel crate directly
imports `arch-aarch64`, hardcodes UART addresses (`0x09000000`), GIC addresses
(`0x08000000`), and embeds scheduler logic inside the architecture crate.

This architecture cannot scale:

- Adding x86_64 requires changing the kernel.
- Adding a new board (RK3568, Lavender) requires modifying kernel code.
- The kernel is an "ARM QEMU kernel", not an architecture-independent kernel.

A Cargo workspace refactoring is already in progress (arch-api, arch-aarch64,
platform-qemu, target-qemu-aarch64 exist as separate crates), but the
dependency graph still contains illegal edges.

## Decision

### 1. Dependency Direction

Dependencies flow **downward** — from composition toward implementation.

```
Target (final binary composition)
 ├── Platform (board/SoC)
 ├── Kernel (architecture-independent logic)
 └── Arch implementation (ISA-specific)

Kernel → arch-api contracts only
Arch implementation → arch-api contracts only
Platform → boot-info
```

### 2. Forbidden Dependencies

```
kernel → arch-aarch64          (kernel must not know the ISA)
kernel → platform-*            (kernel must not know the board)
arch → platform-*              (arch must not know the SoC)
platform → kernel              (platform is pre-kernel)
platform → arch                (platform describes hardware, doesn't drive it)
```

### 3. Stack Ownership

#### Kernel owns:
- Thread lifecycle
- Scheduler policy (which thread, when)
- Process model
- Memory policy (PMM, VMM policy)
- Resource ownership

#### Arch owns:
- CPU registers
- Exception entry/exit save/restore
- Context switching mechanism (assembly)
- MMU implementation (page table format, TLB management)
- ISA-specific instructions (wfi, hlt, etc.)

#### Platform owns:
- SoC description (UART, GIC, timer addresses)
- MMIO location discovery (from FDT, ACPI, or hardcoded)
- Memory map discovery
- Board initialization (console, FDT parsing)

#### Target owns:
- Final executable composition
- Linker script
- System image assembly
- Connecting platform → kernel → arch

### 4. Information Flow

```
Target
  ↓
Platform::initialize()
  ↓
BootInfo { memory_map, mmio_regions, interrupt_controller, ... }
  ↓
kernel_main(&BootInfo)
  ↓
arch_api::* (extern "Rust" functions)
  ↓
arch implementation (AArch64, x86_64, ...)
```

### 5. BootInfo as the Single Contract

`BootInfo` is the **only** information channel from Platform to Kernel.

Kernel must NOT:
- Parse FDT directly
- Hardcode MMIO addresses
- Discover interrupt controllers
- Know UART base addresses

Kernel receives `BootInfo` and acts on it.

`BootInfo` describes "what the kernel needs to exist" — not a full device tree.

## Principles

1. **Hardware first** — hardware-specific properties are isolated behind
   platform/arch boundaries.

2. **No abstraction before second implementation** — do not create
   `trait Architecture` or `trait Uart` until a second ISA/SoC is added.
   Triggers for abstraction TBD per ADR-015.

3. **Dependency direction is architecture** — a crate may only depend on
   crates below it in the layer hierarchy.

4. **Static composition over runtime polymorphism** — the target crate
   selects the arch implementation at build time. No runtime dispatch.

## Consequences

- `kernel/Cargo.toml` will lose its dependency on `arch-aarch64`.
- `kernel/src/lib.rs` will lose all `#[cfg(target_arch = "aarch64")]`
  blocks.
- Platform address constants (0x09000000, 0x08000000) will be removed
  from kernel.
- Scheduler policy will move from `arch-aarch64::thread` to
  `kernel::scheduler`.
- `arch_state: usize` will replace concrete `ExceptionFrame`/`ThreadContext`
  in the kernel's `Thread` struct.
- Adding x86_64 will not require any changes to `kernel/`.