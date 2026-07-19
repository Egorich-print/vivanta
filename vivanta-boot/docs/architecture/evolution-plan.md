# Vivanta Architectural Evolution Plan

## 0. M4 — Execution Foundation (2026-07-16) — Current State

> Status: ✅ **Complete**. Base tag: `M4` (amended commit `37172c3`).

### M4.0 – M4.3: Kernel Thread Environment

- **Cooperative round-robin scheduling** (3 threads: boot + persistent + terminating)
- **Thread lifecycle**: `create_kernel_thread`, `thread_exit`, `thread_trampoline`, `cleanup()`, idle WFI
- **Timer at ~79 Hz** on QEMU (CNTP, IRQ 30, tick counting)
- **Architecture-independent kernel** (verified by `target-test`)
- **Repository restructuring**: `boot/` → `archive/`, `kernel/src/memory/` → `kernel-memory-frozen/`
- **5 ADRs** (ADR-011 through ADR-015)

### M4.4 — Address Spaces (2026-07-17) ✅

> Status: ✅ **Complete**.

**Architecture** — Multi-address-space kernel with verified hardware isolation:

| Component | File | Description |
|-----------|------|-------------|
| `AddressSpace` | `kernel/src/vmm/address_space.rs` | id, root table handle, mapping set, flags (Kernel/User) |
| `RootPageTable` | `arch-api/src/mmu.rs` | Opaque `usize` handle — no TTBR/CR3/SATP in kernel |
| `MappingSet` | `kernel/src/vmm/mapping.rs` | `Mapping` + `VirtRange` to describe VA→PA bindings |
| `KernelAddressSpace` | `kernel/src/vmm/address_space.rs` | Singleton at id 0, initialised at boot |
| Address Space Registry | `kernel/src/vmm/address_space.rs` | Static 8-slot array, `register()` returns `AddressSpaceId` |
| `activate_address_space` | `arch-api/src/mmu.rs` + `arch-aarch64/src/mmu.rs` | TTBR0_EL1 ← root, DSB, TLBI VMALLE1IS, ISB |
| Scheduler activation | `kernel/src/scheduler/mod.rs` | `yield_now()` and `thread_exit()` call `activate_address_space()` on AS mismatch |

**Key design decisions:**
- `AddressSpace` is **opaque** to the scheduler — only an `AddressSpaceId` travels with `Thread`
- `RootPageTable` is opaque to the kernel — arch-api hides the ISA-specific register (TTBR0 on AArch64, CR3 on x86_64)
- AS activation is unconditional on every switch that changes AS — no lazy or deferred TLB invalidations

**Experimental proof — D1 through D3:**

| Test | Setup | Result |
|------|-------|--------|
| D1 | Boot thread (AS 0) + UserAS1 thread (AS 1) + UserAS2 thread (AS 2), cooperative round-robin | Three ASes switch stably; `[AS switch]` trace confirms every transition |
| D2 | Independent root tables for each AS (identical identity-mapped RAM/MMIO) | Root tables at different physical addresses (0x40100000, 0x40105000, 0x4010a000); system stable |
| D3.1 | UserAS1 gets one extra mapped page (PA 0x402xxxx → VA 0x60000000); UserAS2 does not | UserAS1 thread reads/writes the page successfully across context switches |
| D3.2 | UserAS2 thread reads the same VA 0x60000000 | **Data Abort** — ESR=0x96000006 (Translation fault, level 2), FAR=0x60000000 |
| D3.3 | Fault handler prints register dump and halts | Diagnostic output confirms the fault origin and address |

D3.2 is the critical milestone: it is the **first hardware-confirmed isolation test** in vivanta-boot. The MMU enforced a different VA→PA mapping per address space, and the fault information (ESR_EL1, FAR_EL1) was accurate.

**Feature gate:** Address space switching can be traced at runtime with `trace-address-space`:
```sh
cargo build -p target-qemu-aarch64 --features kernel/trace-address-space
```

### M4.4.5 — Execution Contract Freeze (2026-07-17) ✅

> Status: ✅ **Complete**.

M4.4.5 is an **architectural stabilization milestone**, not a feature milestone.
It fixes the execution model before transitioning to EL0 (M4.5).

**ADR-017** (Unified Execution Context) supersedes ADR-012:

| Decision | Before (ADR-012) | After (ADR-017) |
|----------|-------------------|------------------|
| Context switch mechanism | Dual-path: `context_switch_coop` + `context_switch_preempt` | Single `context_switch()` — unified for all cases |
| ExceptionFrame ownership | Copied between thread stacks on preemption | Never copied — frame lives on the owning thread's stack only |
| Execution privilege | Implicit (hardcoded SPSR) | Explicit `ExecutionLevel` enum (`Kernel` / `User`) |
| Interrupt control | Inline `DAIFSet`/`DAIFClr` asm in kernel | `arch_api::interrupts::InterruptGuard` — RAII, arch-provided |

**Additional changes:**
- `ArchContext` is now an opaque `#[repr(transparent)] struct` (was `pub type ArchContext = usize`)
- Kernel `IrqGuard` removed — uses `arch_api::interrupts::disable_interrupts()`
- No architecture-specific inline assembly remains in the kernel crate
- Compile-time layout assertions for `ExceptionFrame` and `ThreadContext`
- Test stub (`target-test`) updated for the new API

**Status after M4.4.5:** Preemptive context switching is architecturally possible
but not yet enabled (timer-driven reschedule path is prepared but inactive).
Cooperative switching is unchanged and stable.

### M4.5.0 — EL0 Transition Preparation (2026-07-17) ✅

> Status: ✅ **Complete.** Tag: `M4.5.0-el0-preparation`

| Change | File | Description |
|--------|------|-------------|
| InterruptGuard state preservation | `arch-api/src/interrupts.rs` | `saved_daif: usize`, restore exact DAIF (not unconditional DAIFClr) |
| `user_stack_top` parameter | `arch-api/src/context.rs` | `context_init` accepts `user_stack_top` (0 for kernel threads) |
| x30 routing | `arch-aarch64/src/context.rs` | Kernel → entry, User → `eret_to_user_stub` |
| `eret_to_user_stub` | `arch-aarch64/src/user.rs` | SP_EL1-relative synthetic frame → eret (unreachable until M4.5.1) |
| EL0 Transition Audit | `docs/architecture/milestones/M4.5.0-el0-preparation.md` | A1-A7 checklist (offsets, SP split, no accidental user paths) |

**Key result:** The execution contract is ready for user threads. `eret_to_user_stub`
compiles but is dead code until M4.5.1.

### M4.5.1 — First EL0 Execution & SVC Roundtrip (2026-07-17) ✅

> Status: ✅ **Complete.** Tag: `M4.5.1-el0-execution`

| Change | File | Description |
|--------|------|-------------|
| ADR-019 | `docs/adr/ADR-019-user-page-permissions.md` | PageFlags with PXN/UXN, TTBR0-only deferral, EL0 ownership table |
| Removed `UserBootstrap::enter()` | `arch-aarch64/src/user.rs` | Deleted duplicate EL0 entry path (inline asm eret) |
| PXN support | `arch-aarch64/src/mmu.rs` | `ENTRY_PXN = 1 << 53`, `privileged_executable` field, user pages → PXN=1 |
| User code + SVC handler | `arch-aarch64/src/user.rs` | `mov x0,#42; svc; mov x0,#43; svc; b .` — two-call roundtrip proof |
| `create_user_thread()` | `kernel/src/scheduler/mod.rs` | Scheduler creates `Thread` with `ExecutionLevel::User` |
| User page mapping | `kernel/src/lib.rs` | UserAS1 gets code+stack during boot via `mmu_map_user_pages()` |
| SVC handler output | `arch-aarch64/src/user.rs` | `boot_common::println!("  SVC from EL0: x0={}", val)` |

**Key results:**
- Full EL1→EL0→EL1→EL0 roundtrip verified (two consecutive SVC calls)
- `eret_to_user_stub` is the single EL0 entry path (ADR-018 Invariant 8 enforced)
- PXN prevents EL1 execution of user pages (ADR-019 §2)
- Architecture score: 9/10

### M4.6 — User Isolation & Syscall Boundary (Planned)

> Status: 🔲 **Planned.** Preceded by ADR-020 (User Process Model).

| Phase | Scope |
|-------|-------|
| M4.6.0 | ADR-020 — Thread vs Process ownership, freeze user execution model |
| M4.6.1 | Syscall ABI (x8 = number, x0-x5 args, x0 return), `sys_yield` first syscall |
| M4.6.2 | Safe `sys_write` with `copy_from_user`, user pointer validation |
| M4.6.3 | TTBR1 kernel split (optional, after syscall ABI stable) |

Key architectural question: will Vivanta use a classical Process model (Variant A)
or a capability-based Execution Object model (Variant B, closer to seL4/Fuchsia)?

## 0.1 Architecture Cleanup Sprint (2026-07-14) — Historical

The Architecture Cleanup Sprint reorganized Vivanta from a monolithic ARM kernel to a
layered architecture with strict dependency direction.

### 0.1.1 Repository Structure (post-sprint)

```
vivanta-boot/
├── boot-info/              # BootInfo, MemoryMap, MmioRegion (core-only, zero deps)
├── boot_common/            # Console, println!, FDT scanner, NS16550
├── arch-api/               # extern "Rust" declarations + MappingFlags (core-only)
├── arch-aarch64/           # AArch64 MMU, GIC, timer, context switching
├── arch-armv7a/            # Frozen stub
├── kernel/                 # Architecture-independent: PMM, scheduler, VMM
├── platform-qemu/          # QEMU virt: PL011 UART, memory discovery
├── platform-rk3568/        # RK3568: NS16550 UART, memory discovery
├── platform-sdm660/        # SDM660: MSM UART
├── target-qemu-aarch64/    # Final binary: QEMU AArch64
├── target-qemu-armv7a/     # Final binary: QEMU ARMv7-A
├── target-rk3568/          # Final binary: Rockchip RK3568
├── target-lavender/        # Final binary: SDM660 (Lavender)
├── arch-test-stub/         # Test stub for build-time proof
├── target-test/            # Proof binary linking kernel + stub
├── boot/                   # Legacy boot adapters (to be migrated)
└── docs/adr/               # ADR-011 through ADR-015
```

### 0.1.2 Dependency Graph (post-sprint)

```
kernel → arch-api, boot-common, boot-info        # NO arch-aarch64
arch-aarch64 → arch-api, boot-common             # NO kernel
platform-qemu → boot-common, boot-info           # NO kernel, NO arch
target-qemu-aarch64 → kernel, arch-aarch64, platform-qemu, boot-common
```

### 0.1.3 Communication Mechanism

Architecture operations cross crate boundaries via `extern "Rust"` declarations in
`arch-api`. Each arch implementation (arch-aarch64) provides `#[no_mangle]` functions.

Kernel calls into arch: `arch_api::boot::mmu::mmu_init(...)`, etc.
Arch calls into kernel: `arch_api::scheduler::scheduler_tick()`, etc.
Both directions use the same `extern "Rust"` mechanism — no traits, no C ABI.

### 0.1.4 Scheduler Split

- **Kernel owns**: Thread lifecycle, RunQueue, scheduling policy, yield_now
- **Arch owns**: Context switching mechanism (context_switch_asm), exception frame layout
- Bridge: `ArchContext = usize` (opaque handle), `InterruptFrameHandle = usize`

### 0.1.5 Build-Time Proof

`target-test` links `kernel` + `arch-test-stub` without any real architecture crate,
proving the kernel does not depend on any specific ISA.

## 1. Pre-Sprint Architecture Report (Historical Reference)

### 1.1 Repository Structure

```
vivanta-boot/                          # Workspace root (Cargo workspace)
├── .cargo/config.toml                 # Default target: aarch64-unknown-none
├── Cargo.toml                         # Workspace with 6 members
├── build.sh                           # QEMU launch (aarch64 kernel mode)
│
├── boot_common/                       # Shared kernel boot protocol types
│   ├── lib.rs                         # Architecture, BootSource, MemoryRegion/Map,
│   │                                  #   BootInfo, Console trait, GlobalConsole,
│   │                                  #   FmtAdapter, print!/println!
│   └── ns16550.rs                     # NS16550 UART driver (NEW, Stage 0)
│
├── boot/
│   ├── aarch32/qemu_virt/             # ARMv7-A QEMU virt (PL011, kernel_main)
│   ├── aarch64/qemu_kernel/           # AArch64 QEMU virt (PL011, kernel_main)
│   ├── aarch64/lavender/              # Qualcomm SDM660 (PAUSED — ABL issue)
│   └── platforms/rk3568/              # Rockchip RK3568 (Stage 0-1, NS16550)
│
├── kernel/
│   ├── lib.rs                         # kernel_main() — boot → init
│   ├── pmm.rs                         # PmmBitmap, BootMemoryManager
│   ├── mmu.rs / mmu/{aarch64,armv7}   # Page table builders
│   ├── vmm/                           # VMM-0 stubs + KernelAddressSpace
│   └── memory/                        # MemoryObject, MRM, Capability, Policy
│
├── docs/architecture/
│   ├── principles.md                  # Hardware Transparency principle
│   ├── memory-geometry.md             # Multi-page-size audit
│   └── address-space.md              # VA layout proposal
├── docs/memory_architecture.md       # Resource-oriented memory model
└── tests/minimal_test.rs             # Minimal QEMU blinky
```

### 1.2 Module Dependencies

```
boot_common (no_std, no arch deps)
  ↑            ↑            ↑
  │            │            └── kernel (kernel_main, PMM, MMU, MemoryObject, MRM)
  │            │
  │            ├── boot/aarch64/lavender    (text_offset=0x80000, MSM UART)
  │            ├── boot/platforms/rk3568    (text_offset=0, NS16550)
  │            └── boot/aarch64/qemu_kernel (full stack: FDT + kernel_main)
  │
  └── boot/aarch32/qemu_virt       (same as qemu_kernel but ARMv7)
```

### 1.3 Implemented Components

| Component | Status | Details |
|-----------|--------|---------|
| BootInfo (RFC-009) | ✅ | Architecture, MemoryMap, DTB, CPU count |
| MemoryMap | ✅ | 16 regions, Usable/Reserved/MMIO kinds |
| Console trait | ✅ | `write_str(&self, s: &str)` |
| print!/println! macros | ✅ | Global via `GLOBAL_CONSOLE` |
| FDT scanner (QEMU) | ✅ | Parses `/memory`, model, compatible |
| Platform trait | ⚠️ | Duplicated mod.rs in two boot crates |
| PL011 UART (QEMU) | ✅ | Full init + TX |
| NS16550 UART | ⚠️ | 8-bit access — needs 32-bit (reg-shift=2) |
| MSM UART (Lavender) | ⚠️ | 32-bit access, not NS16550-compatible |
| ARM64 Image header | ⚠️ | Needs layout fix for U-Boot compat |
| ARM64 entry (stack, BSS, FP) | ✅ | EL-level detection, branch to Rust |
| ARMv7 entry | ✅ | entry.S, BSS zero, FP enable |
| PmmBitmap | ✅ | Frame-level bitmap allocator |
| BootMemoryManager | ✅ | Kernel + DTB + bitmap reservation |
| AArch64 page tables | ✅ | L1-L3, 4K/2M mapping |
| ARMv7 page tables | ✅ | L1 sections, 16KB-aligned |
| PageTableGuard (MMU enable) | ✅ | MAIR/TCR/TTBR0/SCTLR setup |
| VMM-0 stubs | ⚠️ | Placeholder API (map/unmap/protect) |
| KernelAddressSpace | ⚠️ | Placeholder geometry |
| PageFaultHandler trait | ✅ | Trait + PanicHandler |
| MemoryObject | ✅ | State machine (Created→Allocated→Mapped→Revoked) |
| MemoryBackend trait | ✅ | allocate/deallocate/properties |
| MemoryResourceManager | ✅ | Backend registry + policy-based selection |
| MemoryCapability | ⚠️ | No enforcement (check() returns true) |
| PlacementPolicy (scoring) | ✅ | Fastest/Largest/Persistent/Balanced |
| PmmMemoryBackend adapter | ✅ | PMM as a MemoryBackend |
| Hardware Transparency | 📄 | Documented principle (not enforced in code) |
| Memory Geometry | 📄 | Documented multi-page-size plan |
| VA address space layout | 📄 | Provisional layout documented |

### 1.4 Architecture Debt

#### Critical (Blocks hardware bring-up)

1. **NS16550 register access width**: RK3568 DTS specifies `reg-io-width = <4>; reg-shift = <2>`. Current driver uses byte accesses (`base.add(5)`). Must use `*(volatile u32*)(base + (reg << 2))`. **This is why UART outputs garbage on RK3568.**

2. **ARM64 Image header**: U-Boot v2017.09 struct expects magic at offset 56 (64-byte header). Our header has magic at offset 52 (60-byte header). Need exact match with `{ code0 + code1 + 6× .quad + magic + res5 }` for 64 bytes. **Without this, `booti` will reject the image.**

3. **x0 (DTB) not preserved**: Entry code in rk3568/lavender does not save x0 before branching to Rust. Stage 1.5 requires this for FDT-based platform detection.

#### Structural (Duplication)

4. **FDT scanner duplicated**: `boot/aarch32/qemu_virt/src/fdt.rs` and `boot/aarch64/qemu_kernel/src/fdt.rs` are identical. Should move to `boot_common` or a new `fdt` crate.

5. **Platform trait duplicated**: `platform/mod.rs` + `platform/qemu.rs` exist in both aarch32 and aarch64 QEMU crates. Identical PL011 UART code duplicated.

6. **Entry code duplicated**: EL-level detection, BSS zero, stack setup — repeated 3× (lavender, rk3568, qemu_kernel). Should be in a shared `boot/arch/aarch64/entry.S`.

#### Design debt

7. **Platform trait**: Currently `Platform { fn console(&self) -> &dyn Console }` — too narrow. No timer, no reset, no watchdog.

8. **MemoryCapability.check()**: Always returns `true` — no enforcement.

9. **VMM-0 stubs**: `map/unmap/protect/translate` are all `unimplemented!()`. The only working path is `PageTableBuilder` (direct MMU manipulation bypassing VMM). MemoryObject lifecycle calls `map()` on the object, but this does NOT actually modify page tables.

10. **Hardcoded page constants**: 33 hardcoded `4096`/`0x1000` references across PMM and MMU code (documented in memory-geometry.md).

11. **policy.rs scoring**: Good abstraction, but the PMM adapter allocates only 1 frame per call while `MemoryObject.map()` expects the full object size.

12. **Documentation vs code gap**: `principles.md` describes Hardware Transparency, but `PageTableBuilder` still hardcodes AArch64-specific page table layout. VMM stubs don't use `MemoryGeometry`.

---

## 2. Future Architecture Proposal

### 2.1 Code Organisation

```
vivanta-boot/
│
├── boot_common/                      # ★ Expand
│   ├── lib.rs                        #   Console, Architecture, BootInfo, MemoryMap
│   ├── ns16550.rs                    #   NS16550 UART driver (32-bit fixed)
│   └── fdt.rs                        # ← MOVE from boot/qemu adapters
│
├── boot/
│   ├── arch/
│   │   ├── aarch64/
│   │   │   ├── entry.S               # ← EXTRACT common ARM64 entry
│   │   │   ├── start.rs             #   EL detection, stack, BSS → Rust call
│   │   │   └── link.ld              #   Generic AArch64 linker script
│   │   └── armv7/
│   │       ├── entry.S
│   │       └── link.ld
│   │
│   ├── drivers/
│   │   ├── uart/
│   │   │   ├── mod.rs               #   EarlyConsole trait (polled TX)
│   │   │   ├── ns16550.rs           #   Generic NS16550 backend
│   │   │   ├── pl011.rs             #   Generic PL011 backend
│   │   │   └── msm_dm.rs            #   Qualcomm DM UART backend
│   │   └── timer/                   #   Future: ARM Generic Timer, etc.
│   │
│   ├── platforms/
│   │   ├── rk3568/                   #   Platform-specific: UART base, FDT ref
│   │   ├── lavender/                 #   (PAUSED)
│   │   └── qemu/                     #   QEMU virt (aarch64 + armv7)
│   │
│   └── adapters/                     #   Existing boot crates (thin wrappers)
│       ├── aarch64-qemu-kernel/
│       ├── aarch32-qemu-virt/
│       └── aarch64-rk3568/
│
├── kernel/
│   ├── src/
│   │   ├── lib.rs                    # kernel_main()
│   │   ├── pmm.rs                    # PmmBitmap (use MemoryGeometry)
│   │   ├── mmu.rs / mmu/             # Page table builders
│   │   ├── vmm/                      # VMM-1: real map/unmap
│   │   └── memory/                   # MemoryObject, MRM, Policy
│   │
│   └── arch/                         # ← NEW: arch-specific kernel support
│       ├── aarch64/
│       │   ├── page_table.rs         #   AArch64 page table implementation
│       │   └── cpu.rs                #   CPU feature detection, cache ops
│       └── armv7/
│           ├── page_table.rs
│           └── cpu.rs
│
└── docs/
    ├── architecture/
    │   ├── principles.md
    │   ├── memory-geometry.md
    │   ├── address-space.md
    │   ├── evolution-plan.md         # ← THIS FILE
    │   └── hardware-graph.md         # ← NEW: Hardware Graph design
    └── rfcs/                         # ← NEW: formal RFCs
```

### 2.2 Key Architectural Changes

#### A. KernelObject Model (Deferred to M5)

Proposed common kernel object abstraction is **not yet needed**. Current abstractions (MemoryObject, DeviceObject planned) have divergent lifecycles. Premature unification risks overengineering.

**Decision**: Keep MemoryObject standalone. Revisit when 3+ object types exist and a common pattern emerges.

#### B. DeviceObject + Hardware Graph (M4 target)

Replace direct FDT dependence after boot:

```
FDT / ACPI
    ↓
BootInfo Builder (M2)
    ↓
Hardware Graph (M4)
    ├── Node: each device (CPU, UART, GIC, memory controller)
    ├── Edges: bus topology (AMBA, PCIe, platform bus)
    └── Properties: reg, interrupts, clocks, power
    ↓
DeviceObject
    ├── Identity derived from DTB path
    ├── State: Discovered → Authorized → Active → Suspended
    ├── Driver binding via compatible strings
    └── Capability-based access control
```

**Stage 2 (next)**: Parse FDT → print model/compatible/memory/cpus — without yet building the full graph. Stage 2.5 extracts `PlatformDescriptor` from DTB. Full graph is M4.

#### C. DriverObject (M4-M5)

Driver framework:

```
UartFramework (trait)
    ├── Ns16550Backend (reg-shift, reg-io-width parameterized)
    ├── Pl011Backend
    └── MsmDmBackend
```

Each backend implements `UartFramework { write_byte(), try_read(), init(baud, params), ... }`.

Platform detection (Stage 2.5) selects the appropriate backend based on DTB `compatible`.

#### D. Dynamic Hardware Authorization (M5+)

Deferred. Not needed until userspace drivers exist.

#### E. Application Compatibility (M6+)

Deferred. Linux ELF loader + libc shim layer would be needed. Current focus is kernel bring-up.

### 2.3 Driver Interface Design (Immediate)

```rust
/// Polled UART driver for early boot (no interrupts, no DMA).
pub trait EarlyConsole: Sync {
    fn write_byte(&self, byte: u8);
    fn write_str(&self, s: &str) {
        for &b in s.as_bytes() {
            match b {
                b'\n' => { self.write_byte(b'\r'); self.write_byte(b'\n'); }
                c => self.write_byte(c),
            }
        }
    }
}
```

`boot_common::Console` is kept as the higher-level trait. `EarlyConsole` is the low-level interface for drivers. A blanket impl converts `EarlyConsole` → `Console`.

```rust
// In boot_common:
impl<T: EarlyConsole + Sync> Console for T {
    fn write_str(&self, s: &str) {
        EarlyConsole::write_str(self, s);
    }
}
```

---

## 3. Migration Plan

### Phase 0: Documentation & Interface Freeze (NOW)

1. Accept this evolution plan.
2. Document NS16550 register access requirement.
3. Document U-Boot Image header struct requirements.
4. Create `boot/drivers/` interface stubs.

### Phase 1: Fix before refactor (1-2 sessions)

1. **Fix NS16550 driver**: 32-bit access with `reg-shift = <2>`.
2. **Fix ARM64 Image header**: 64-byte layout matching U-Boot struct.
3. **Preserve x0** in entry code.
4. **Build + test** on RK3568 with U-Boot.

### Phase 2: Extract shared boot code (2-3 sessions)

1. FDT scanner → `boot_common`.
2. Platform trait + PL011 → shared location.
3. ARM64 entry.S → `boot/arch/aarch64/`.
4. Linker scripts → `boot/arch/aarch64/`.
5. Adapt all 4 boot crates to use shared code.

### Phase 3: Introduce new abstractions (3-4 sessions)

1. `EarlyConsole` trait.
2. `boot/drivers/uart/` with ns16550, pl011 backends.
3. `MemoryGeometry` → PMM + MMU integration.
4. VMM-1: real `map()`/`unmap()` implementations.
5. MemoryCapability enforcement (optional now).

### Phase 4: Hardware expansion (ongoing)

1. RK3568 Stage 2 (FDT → BootInfo).
2. RK3568 Stage 2.5 (PlatformDescriptor).
3. RK3568 Stage 3 (PMM on real hardware).
4. Multi-platform support: QEMU + RK3568.
5. Lavender SDM660 (conditional on UART access).

---

## 4. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Overengineering | Medium | Low | Phase 0 design review; defer abstract concepts until 3+ instances exist |
| U-Boot image header incompatibility | High | High (boot fails) | Phase 1 fix; test with real booti immediately |
| UART register width wrong | High | High (no output) | Phase 1 fix; 32-bit access with shift=2 |
| FDT scanner panics on real DTB | Medium | Medium (no memory map) | Phase 2 adds real DTB validation; Stage 2 tests with board DTB |
| Lavender ABL blocking | Certain (known) | Low | Pivoted to RK3568; revisit when/if UART access solved |
| MemoryObject/VMM gap | Medium | High (no real MMU enable) | VMM-1 implementation in Phase 3 closes this gap |

---

## 5. Refactoring Roadmap

### Session 1: Phase 1 — Critical bug fixes

Files to modify:
- `boot_common/src/ns16550.rs` — 32-bit access with `(reg << 2)` offset
- `boot/platforms/rk3568/src/main.rs` — 64-byte Image header, preserve x0
- `boot/aarch64/lavender/src/main.rs` — same header fix

### Session 2: Phase 2 — Extract shared boot code

Files to create:
- `boot/arch/aarch64/entry.S` — common ARM64 entry (stack, BSS, x0 save)
- `boot/arch/aarch64/link.ld` — common linker script
- `boot_common/src/fdt.rs` — FDT scanner (moved from qemu crates)
- `boot_common/src/early_console.rs` — EarlyConsole trait

Files to modify:
- `boot_common/src/lib.rs` — add modules
- All 4 boot crates — use shared entry + linker script

### Session 3: Phase 3 — New abstractions

Files to create:
- `boot/drivers/uart/mod.rs` — EarlyConsole trait + builder
- `boot/drivers/uart/ns16550.rs` — moved from boot_common
- `boot/drivers/uart/pl011.rs` — moved from qemu platforms

Files to modify:
- PMM + MMU: use MemoryGeometry instead of hardcoded 4096
- VMM: implement real map/unmap
- MemoryObject: connect map() to actual VMM

### Sessions 4+: Hardware bring-up

- Stage 2: FDT → BootInfo validation (save x0, verify magic, print model)
- Stage 2.5: PlatformDescriptor from DTB
- Stage 3: BootInfo + PMM on RK3568
- Stage 4: MMU enable on RK3568

---

## 6. Appendix: Audit Details

### A. NS16550 Register Access (Critical Fix)

```rust
// CURRENT (broken for RK3568):
fn thr(&self) -> *mut u8 { self.base }           // byte access, offset 0
fn lsr(&self) -> *mut u8 { unsafe { self.base.add(5) } }  // byte access, offset 5

// REQUIRED (RK3568 DTS: reg-shift=<2>, reg-io-width=<4>):
fn thr(&self) -> *mut u32 { self.base as *mut u32 }                    // offset 0, 32-bit
fn lsr(&self) -> *mut u32 { unsafe { (self.base as *mut u32).add(5) } } // offset 5<<2=20, 32-bit
```

### B. U-Boot Image Header (Critical Fix)

U-Boot v2017.09 expects:

```
Offset  Size  Field         Notes
0       4     code0         branch instruction (b _start)
4       4     code1         branch second word (0 for ARM64)
8       8     text_offset   LE, must be 0 for PIE kernels
16      8     image_size    LE, 0 → U-Boot assumes 16MiB
24      8     flags         LE: bit3=1 → PIE (phys offset anywhere)
32      8     res2          0
40      8     res3          0
48      8     res4          0
56      4     magic         0x644d5241 ("ARMd")
60      4     res5          0
                        Total: 64 bytes
```

Magic check in `booti_setup()`:
```c
struct Image_header *ih = (struct Image_header *)image;
if (ih->magic != le32_to_cpu(0x644d5241))
    return 1;  // BAD — our current header fails here
```

### C. Possible Duplication Inventory

| File | Occurrences | Recommendation |
|------|-------------|---------------|
| `fdt.rs` | 2 (aarch32, aarch64 qemu) | Move to boot_common |
| `platform/mod.rs` | 2 | Extract Platform trait to boot_common |
| `platform/qemu.rs` (PL011) | 2 | Move to boot/drivers/uart/pl011.rs |
| ARM64 entry code | 3 (lavender, rk3568, qemu_kernel) | Share via boot/arch/aarch64/entry.S |
| Linker scripts | 4+ | Create arch-generic template |

### D. Non-Critical Issues

7. `policy.rs` scoring weights: arbitrary values, no real hardware data to tune.
8. `MemoryObject.clone()` — lacks Copy-on-Write; both objects share same backend pages.
9. `PmmMemoryBackend.allocate()` — single frame only, ignores alignment.
10. Bootstrap procedure in `kernel/src/lib.rs`: bitmap allocated after `__stack_top` — works but fragile; should be derived from BootInfo.
**Future Vision:** Introduce a protocol-neutral Network Service Framework where Reticulum becomes the first reference implementation rather than a kernel networking subsystem. See `docs/rfc/network-services-vision.md`.
