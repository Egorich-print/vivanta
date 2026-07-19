> **Note:** At the time of writing, the project was named TheseusOS. The content reflects the historical state and is preserved as-is.
>
# ADR-011: Phase Transition — Research Prototype → Engineering Platform

## Status

Accepted

## Date

2026-07-13

## Context

Vivanta began as an architecture-first research prototype. It produced:

- ARM64 boot infrastructure (Image header, EL detection, stack, BSS, FP enable)
- QEMU virt boot adapters for ARMv7 and AArch64
- PMM bitmap allocator with BootMemoryManager
- AArch64 page table builder with MMU activation
- FDT scanner for basic memory discovery
- MemoryObject state machine (Created → Allocated → Mapped → Shared → Revoked)
- MemoryResourceManager with policy-based backend selection
- MemoryCapability with deferred enforcement
- PlacementPolicy scoring engine
- BootContext / BootInfo / Console / print! infrastructure
- RK3568 boot adapter with NS16550 UART (reg_shift=2 fixed)
- BootContext with x0 (DTB) preservation
- U-Boot-compatible ARM64 Image header (64-byte, magic at offset 56)

Three independent architecture reviews (ChatGPT, Gemini, Grok) converged on the
same critical finding:

> The project is implementing advanced abstractions (MemoryObject, Capabilities,
> Policy engine, DeviceObject) before validating fundamental OS capabilities
> (interrupts, timer, scheduler, userspace, real VMM) on physical hardware.

The abstractions are well-designed but premature. They create false progress:
code that looks finished but cannot be tested, validated, or exercised on real
hardware.

The project is NOT abandoning advanced architecture. It is changing the order
of implementation.

---

## Decision

### Development philosophy

**Hardware first. Validated primitives before abstractions.**

Every layer must be physically validated before the next layer is built.
Advanced abstractions are preserved as RFC design records but removed from the
active implementation until their preconditions exist.

### Engineering rule

> **No abstraction before second implementation.**

Before creating a trait, interface, or generic abstraction:

1. Two independent implementations must exist in the codebase
2. Common behavior must be demonstrable (not speculative)
3. The interface must reduce measurable duplication
4. The interface must be tested against both implementations

Otherwise: keep local implementations. Do not abstract.

### What changes

**Keep active** — required for current milestone:

- BootContext / BOOT_CONTEXT — DTB handoff
- BootInfo — typed kernel entry data
- Console trait + print!/println! — stable, used everywhere
- MemoryMap / MemoryRegion — PMM dependency, stable
- PMM (PmmBitmap + BootMemoryManager) — working allocator
- AArch64 PageTableBuilder — working MMU code
- NS16550 driver — required for RK3568
- ARM64 Image header — required for U-Boot
- HardwareNode (Stage 1) — minimal FDT→driver IR

**Freeze into RFC** — preserve design, remove from active build:

- MemoryObject — lifecycle state machine (RFC-012)
- MemoryCapability — unenforced capability checks (RFC-013)
- MemoryResourceManager — policy-based backend selection (RFC-013)
- PlacementPolicy — scoring engine (RFC-016)
- PmmMemoryBackend adapter — PMM as MemoryBackend (RFC-015)
- MemoryBackend trait + MemoryProperties (RFC-015)
- ARMv7 MMU implementation (frozen, not deleted)

**Reduce to structure** — remove stubs, keep minimal skeleton:

- VMM stubs (map/unmap/protect/translate → empty AddressSpace struct)
- VMM address_space.rs → single struct with no methods
- VMM faults.rs → keep PanicHandler only, remove PageFaultHandler trait

**Remove from active build** — dead code / false progress:

- MemoryObject lifecycle demo in kernel_main()
- ARMv7 MMU from workspace build
- aarch32 qemu_virt from active development

---

## Architecture layers

```
Layer 9  │ Advanced Architecture
         │ MemoryObject, Capability System, Hardware Graph
         │ Tiered Memory, Placement Policy
         ════════════════════════════════════════
Layer 8  │ Device Architecture
         │ Driver API, Driver Components
         ════════════════════════════════════════
Layer 7  │ Compatibility
         │ POSIX subset, libc port
         ════════════════════════════════════════
Layer 6  │ Userspace
         │ ELF loader, Syscall ABI, init process
         ════════════════════════════════════════
Layer 5  │ Memory Virtualization
         │ VMM (mmap/munmap/mprotect), Address Spaces
         ════════════════════════════════════════
Layer 4  │ Kernel Execution
         │ Scheduler, Threads, Context Switch
         ════════════════════════════════════════
Layer 3  │ Hardware Runtime
         │ GICv3, ARM Generic Timer, Exceptions
         ════════════════════════════════════════
Layer 2  │ Hardware Discovery
         │ FDT Scanner, Hardware Descriptor IR
         ════════════════════════════════════════
Layer 1  │ Boot
         │ U-Boot handoff, BootContext, Console, PMM, MMU
```

**Rule**: Higher layers may depend on lower layers. Lower layers must never
depend on higher layers.

---

## Active systems

| System | File(s) | Reason |
|---|---|---|
| BootContext + BOOT_CONTEXT | `boot_common/src/lib.rs` | DTB handoff from bootloader |
| BootInfo | `boot_common/src/lib.rs` | Typed kernel entry data |
| Console trait + macros | `boot_common/src/lib.rs` | Debug output |
| MemoryMap + MemoryRegion | `boot_common/src/lib.rs` | PMM input, stable |
| PMM (PmmBitmap + BootMemoryManager) | `kernel/src/pmm.rs` | Only physical allocator |
| AArch64 PageTableBuilder | `kernel/src/mmu/aarch64_impl.rs` | Working MMU code |
| NS16550 driver | `boot_common/src/ns16550.rs` | RK3568 UART |
| ARM64 Image header | all boot adapters | U-Boot compatibility |
| RK3568 boot adapter | `boot/platforms/rk3568/` | Primary hardware target |
| QEMU aarch64 boot adapter | `boot/aarch64/qemu_kernel/` | CI / emulation target |

---

## Frozen systems (archived to RFC, not deleted)

| System | RFC | Preconditions for revival |
|---|---|---|
| MemoryObject | RFC-012 | VMM exists, page fault handling exists, userspace exists |
| MemoryCapability | RFC-013 | Userspace isolation exists, MMU-based enforcement possible |
| MemoryResourceManager | RFC-013 | 2+ backends exist, heterogeneous memory measurable |
| PlacementPolicy | RFC-016 | Multiple backends, real hardware property data |
| MemoryBackend + Properties | RFC-015 | 2+ backend implementations |
| PmmMemoryBackend | RFC-015 | MemoryObject revival |
| ARMv7 MMU | — | ARMv7 target reactivated |
| aarch32 qemu_virt | — | ARMv7 target reactivated |

---

## Rejected for current phase

### Full Hardware Graph

**Reason**: FDT already provides sufficient board-level device discovery. A
graph with lifecycle introduces complexity that only matters when devices are
hot-plugged, power-managed, or isolated — none of which exist yet.

**Status**: Deferred until VMM + driver isolation exist.

### Capability enforcement wrappers

**Reason**: No enforcement boundary exists. Without MMU-based isolation or
userspace/kernel separation, capability checks are decorative. The current
implementation creates false security guarantees.

**Status**: Deferred until userspace isolation exists.

### Memory placement scoring engine

**Reason**: No measurable heterogeneous memory topology exists on the current
target (RK3568 has only DDR4). The scoring weights are arbitrary.

**Status**: Deferred until multiple memory backends can be tested on real
hardware.

### Identity Model (Ed25519, BIP-39, state continuity)

**Note**: This is NOT rejected — it is recognised as one of the most
distinctive Vivanta concepts. It is deferred because:

- It requires secure boot verification
- It requires persistent storage with filesystem
- It requires userspace isolation
- The M1 experiment proved the concept works

**RFC**: RFC-001 (archived) should be revived when Stage 6 (userspace) is
approaching completion.

**Status**: Research / Future Security Architecture. Not frozen — preserved
as design intent but not scheduled for implementation.

---

## Roadmap

```
Stage 0  │ Architecture Reset (this ADR + RFC freeze + cleanup)
         │   - Create ADR and RFC documents
         │   - Remove MemoryObject demo from kernel_main()
         │   - Reduce VMM stubs to empty AddressSpace struct
         │   - Freeze kernel/src/memory/ module
         │   - Verify build + QEMU boot still works
         │
Stage 1  │ RK3568 Physical Boot
         │   - Hardware Descriptor IR (minimal: compatible, reg, irq)
         │   - FDT scanner extract + expand (/memory, /chosen, /cpus)
         │   - FDT-driven UART detection (stdout-path)
         │   - Test on RK3568 via U-Boot booti
         │
Stage 2  │ CPU Exception Infrastructure
         │   - VBAR_EL1, 4 vector entries
         │   - ESR_EL1 / FAR_EL1 / ELR_EL1 / SPSR_EL1 dump
         │   - Stack switching per exception level
         │
Stage 2.5│ Crash Diagnostics
         │   - panic_dump(esr, far, elr, spsr, el)
         │   - Exception class decode (EC from ESR)
         │   - Without this: every MMU/IRQ bug = silent reboot
         │
Stage 3  │ Hardware Runtime
         │   - GICv3 (distributor + redistributor, group 1)
         │   - ARM Generic Timer (CNTPCT, CNTP_CVAL, CNTP_CTL)
         │   - Timer interrupt → GIC → handler
         │
Stage 4  │ Kernel Execution
         │   - Thread struct (context, stack, state)
         │   - Context switch (callee-saved regs + SP/LR)
         │   - Round-robin scheduler
         │   - Idle thread (WFI)
         │
Stage 5  │ Memory Virtualization
         │   - Fill AddressSpace struct
         │   - mmap(phys, virt, size, flags)
         │   - munmap(virt, size)
         │   - mprotect(virt, size, flags)
         │   - Demand paging via fault handler
         │
Stage 6  │ Userspace
         │   - ELF64 loader
         │   - Syscall ABI (SVC handler table)
         │   - Minimal syscalls: write, exit, mmap, brk
         │   - First userspace process
         │
Stage 7  │ Compatibility
         │   - musl libc port (syscall wrappers)
         │   - Minimal _start / exit / write / mmap
         │   - POSIX subset (not Linux clone — macOS/Android model)
         │
Stage 8  │ Device Architecture
         │   - Real drivers in driver form (UART, Timer, GIC)
         │   - Generic Driver API (from validated patterns only)
         │   - Driver binding model
         │
Stage 9  │ Advanced Architecture (revive from RFCs)
         │   - MemoryObject (RFC-012) — backed by real VMM
         │   - Capability Model (RFC-013) — MMU-enforced
         │   - Hardware Graph (RFC-014) — from HardwareNode array
         │   - Tiered Memory (RFC-015) — CXL/VRAM backends
```

### Stage progression rule

A stage is considered complete when:

1. Its primary feature is physically validated on real hardware (RK3568) OR
   in QEMU (for features without physical equivalent)
2. The previous stage has been stable for at least one development session
3. CI build passes for all active targets

---

## Consequences

### Positive

- Physical hardware pipeline is validated before abstractions depend on it
- Each layer has concrete, testable output
- Crash infrastructure catches bugs early (Stage 2.5)
- RFCs preserve design intent without premature implementation
- Architecture debt is explicitly tracked, not hidden in code
- The "second implementation" rule prevents speculative traits

### Negative

- Advanced architecture features are deferred 12-24 months
- MemoryObject / Capability / Hardware Graph exist only as RFCs
- The project will look like "a simple kernel" during Stages 1-6
- May lose contributors who find foundational work less exciting than research

### Risk mitigation

- RFCs are living documents: reviewed every 3 months
- Archive branches preserve frozen code for reference
- ADR-011 is the constitution: any deviation requires a new ADR
- Identity Model (Vivanta's strongest differentiator) is preserved as
  design intent, not frozen

---

## Review criteria

Before advancing to the next stage:

- Physical validation exists (UART, exception, interrupt, timer signals)
- Tests exist (QEMU-based CI)
- Previous stage has been stable
- ADR-011 is still the governing architecture decision

---

## Engineering rules (formal)

### Rule 1: Second implementation

```text
Before creating a trait or generic abstraction:

1. Two independent implementations exist in the codebase
2. Common behavior is demonstrable from those implementations
3. The interface reduces duplication (measured)
4. The interface is tested against both implementations

Otherwise: keep local implementations. Do not abstract.
```

This rule applies to all kernel and driver code. It does not apply to
documentation, RFCs, or design experiments.

### Rule 2: Existing duplication

```text
Every abstraction must remove existing duplication, not anticipated
duplication.
```

A trait, interface, or generic layer may only be introduced to eliminate
duplication that already exists in the codebase. Anticipated future duplication
is not a sufficient reason to abstract.

### Rule 3: Execution context

```text
Every shared mutable state must explicitly document its execution context.
```

Each `static mut` or shared data structure must declare which execution
contexts may access it:

- **boot** — single-threaded, no IRQs, no scheduling
- **thread** — may be preempted by IRQ, may hold locks
- **interrupt** — runs with IRQs disabled, must not block

Example:

```rust
/// Context:
///   Producer: boot
///   Consumer: interrupt
///   Thread:   boot only (written once, read-only after)
static mut GIC_CPU_BASE: u64 = 0;
```

### Rule 4: Stage capability

```text
Every new stage must open a principle new capability, not merely add a new
subsystem.
```

- Stage 1 → load the kernel
- Stage 2 → understand why it crashed
- Stage 3 → respond to external events
- Stage 4 → measure time
- Stage 5 → execute multiple threads
- Stage 6 → safely run user code
- Stage 7 → run real applications

---

## Amendment (2026-07-19): Frozen Component Unfreezing

### Context

Roadmap V2.1 requires integrating the frozen `MemoryObject` implementation (M3-BC, archived per the original ADR-011) into the RK3568 kernel boot path. This is hardware adaptation of already-validated code, not new abstraction design.

### Decision

Frozen components (archived per the original ADR-011) may be unfrozen and modified ONLY when ALL of the following hold:

1. **Hardware necessity** — the existing implementation (if any) cannot satisfy a new hardware target without modification
2. **Documented change** — the modification is recorded in an ADR amendment or new ADR
3. **Regression pass** — tests pass for the original validation environment (QEMU)
4. **Integration, not redesign** — the modification adapts the frozen component to new hardware without changing its architectural intent

If a modification changes an abstraction boundary, a new ADR is required in addition to this amendment.

### Examples

| Case | Status |
|------|--------|
| MemoryObject integration into RK3568 (V2.1) | ✅ Permitted — hardware adaptation of validated QEMU code |
| Adding new allocation policy to PlacementPolicy | ✅ Permitted — existing enum extended, no abstraction change |
| Changing MemoryObject lifecycle state machine | ❌ Requires new ADR — changes validated abstraction boundary |
| Rewriting MemoryCapability for new security model | ❌ Requires new ADR — changes architectural intent |

### Rationale

The original freeze was motivated by "premature abstraction before hardware validation." That precondition is now partially satisfied: MemoryObject was validated on QEMU (M3-BC), and RK3568 hardware exists. The freeze served its purpose — preventing speculative development. Continued freezing would block legitimate porting without providing architectural benefit.

This amendment preserves the spirit of ADR-011: prevent speculative abstractions, not hardware-required porting.
