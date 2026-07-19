# Repository Layout

## Dependency Graph

```
target-*
    ↓
platform-*
    ↓
kernel
    ↕
arch-api  (extern "Rust" contracts)
    ↓
arch-*   (ISA implementations)
    ↓
boot-info  (ABI contracts, zero dependencies)
boot_common  (console, FDT, diagnostics)
kernel-memory-frozen  (RFC prototypes, ADR-011)
archive/boot_legacy/  (pre-ACS adapters, frozen)
```

## Layer Responsibilities

### `target-*` — Composition Layer

Final binaries linking platform + kernel + arch.

```
target-qemu-aarch64/   — QEMU AArch64
target-qemu-armv7a/    — QEMU ARMv7-A
target-rk3568/         — Rockchip RK3568
target-lavender/       — SDM660 (Lavender)
target-test/            — Build-time proof (links arch-test-stub)
```

Responsibility: select platform, select arch, produce the final ELF.

### `platform-*` — Board/SoC Abstraction

```
platform-qemu/    — PL011 UART, FDT-based init
platform-rk3568/  — NS16550 UART, Rockchip-specific
platform-sdm660/  — MSM UART, Qualcomm-specific
```

Responsibility: console init, FDT parsing, memory map construction.
Does NOT know about scheduler, PMM, VMM, or any kernel logic.

### `kernel` — Architecture-Independent Kernel Logic

```
kernel/src/
    lib.rs          — kernel_main(), boot entry
    pmm.rs          — Physical memory manager (bitmap)
    scheduler/      — Thread lifecycle, runqueue, context switching
        mod.rs
        thread.rs
        runqueue.rs
    vmm/            — VMM placeholder (ADR-011, frozen)
```

Responsibility: PMM, scheduler, thread lifecycle, VMM future.
Depends on `arch-api` for ISA abstraction. Never imports `arch-aarch64` directly.
Contains `extern "Rust"` callbacks invoked by arch layers.

### `arch-api` — Architecture Contract Layer

```
arch-api/src/
    lib.rs
    context.rs      — context_init, context_switch_coop, context_switch_preempt
    scheduler.rs    — scheduler_tick(), scheduler_reschedule() callbacks
    pmm.rs          — FrameAllocator trait, PhysFrame
    mmu.rs          — MMU init, map, activate
    boot/           — Boot-time contracts
        cpu.rs      — early_init, wait_for_interrupt
        mmu.rs      — mmu_init, mmu_map_range, mmu_activate
        irq.rs      — irq_init, irq_cpu_enable
        timer.rs    — timer_init, ticks
        sched.rs    — sched_init_boot
        user.rs     — user_bootstrap, user_enter
```

Responsibility: define the kernel ↔ arch boundary via `extern "Rust"`.
Kernel calls arch through `arch_api::boot::*` and `arch_api::context::*`.
Arch calls kernel through `arch_api::scheduler::*`.

### `arch-aarch64` — AArch64 ISA Implementation

```
arch-aarch64/src/
    lib.rs              — init(), is_mmu_enabled()
    boot.rs             — #[no_mangle] impl of all arch_api::boot::*
    context.rs          — context_init, context_switch_coop, idle_entry
    thread.rs           — context_switch_asm (global_asm!)
    timer.rs            — ARM Generic Timer (CNTP), 100 Hz
    exceptions.rs       — ExceptionFrame, exception_handler, ESR/FAR decode
    vectors.rs          — 2048-byte aligned vector table
    interrupts.rs       — enable(), GIC module
    interrupts/
        dispatcher.rs   — IRQ dispatch table (256 entries), irq_entry_handler
        gic.rs          — GICv2/v3 driver
    mmu.rs              — PageTableBuilder (4-level, 4K/2M)
    user.rs             — EL0 bootstrap experiment
    mmio.rs             — MMIO helpers
    barrier.rs          — DSB/DMB/ISB wrappers
    sync.rs             — IrqGuard (DAIF save/restore)
```

Responsibility: implement every `extern "Rust"` function declared in `arch-api`.
Provide ISA-specific primitives (context switch, MMU, GIC, timer, exceptions).
No kernel logic. No scheduler policy.

### `boot-info` — ABI Contract Types

```
boot-info/src/
    lib.rs          — BootInfo struct
    mmap.rs         — MemoryMap, MemoryRegion
    mmio.rs         — MmioRegion
    interrupts.rs   — InterruptControllerInfo
```

Responsibility: define the BootLoader → Kernel ABI contract.
Zero dependencies (only `core`). Used by boot adapters, platform crates, and kernel.
No `std`, no `alloc`.

### `boot_common` — Boot-Time Utilities

```
boot_common/src/
    lib.rs          — Console trait, GlobalConsole, println!, BootContext
    fdt.rs          — FDT scanner (magic, memory, CPU, GIC, console)
    ns16550.rs      — NS16550 UART driver
    hardware.rs     — Architecture, BootSource, MemoryGeometry types
```

Responsibility: shared boot infrastructure — console, FDT parsing, UART drivers.
Used by platform crates, legacy boot adapters, and kernel diagnostics.

### `kernel-memory-frozen` — Memory Resource Model (RFC Prototype)

```
kernel-memory-frozen/src/
    lib.rs          — Re-exports
    object.rs       — MemoryObject lifecycle (Created→Allocated→Mapped→Revoked)
    resource.rs     — MemoryBackend trait, MemoryProperties
    manager.rs      — MemoryResourceManager (backend registry, allocation)
    capability.rs   — MemoryCapability (deferred enforcement)
    policy.rs       — PlacementPolicy scoring engine
```

Responsibility: RFC-012/013/015/016 prototypes.
Frozen by ADR-011 — not compiled into kernel, not used at runtime.
Will be activated when VMM, page faults, and userspace isolation exist.
The `PmmMemoryBackend` adapter (in `kernel/src/`) can be restored when unfrozen.

## Recent Structural Changes

| Change | Rationale |
|--------|-----------|
| `boot/` → `archive/boot_legacy/` | Pre-ACS boot adapters superseded by platform-* + target-* pipeline |
| `kernel/src/memory/` → `kernel-memory-frozen/` crate | RFC prototypes frozen by ADR-011; extracted to prevent bit-rot within kernel |

## Future Directions

### Scheduler (post-M4)

```
scheduler/
    mod.rs          — core dispatch
    thread.rs       — Thread struct
    runqueue.rs     — RunQueue
    idle.rs         — idle thread
    lifecycle.rs    — exit, sleep, wake, join, cleanup
    policy.rs       — scheduling policy
    timer.rs        — timer hooks
```

### arch-aarch64 (post-10k LOC)

```
arch-aarch64/
    lib.rs
    exceptions/
    context/
    interrupts/
    mmu/
    timer/
    boot/
    user/
    sync/
```

## Invariants

1. `kernel` NEVER imports `arch-aarch64` directly — always through `arch-api`.
2. `platform-*` do NOT depend on `kernel` or `arch-*`.
3. `arch-api` contains only `extern "Rust"` declarations — no implementations.
4. `boot-info` has zero dependencies — pure ABI contract.
5. `target-*` is the ONLY layer that directly selects both `platform-*` and `arch-*`.
6. No circular dependencies between workspace crates.
