# Master Roadmap

**Purpose:** This is the single authoritative engineering document defining the development and porting sequence for Vivanta over the next 1–2 years. It does not introduce new specifications, but maps existing architectural components (BootInfo, PMM, VMM, MemoryObject) onto a phased execution plan.

**Authority:** If any implementation plan, TODO list, sprint document, or experimental branch conflicts with this roadmap, this document takes precedence unless explicitly superseded by a newer approved version.

---

## Current Focus

*   **Active Track:** `V-epics P2` (Memory Resource Manager)
*   **Active Milestone:** `V2/M5` (Memory Resource Manager — ADR-025)
*   **Current Engineering Objective:** Integrate existing `MemoryObject` prototype from `kernel-memory-frozen` into the kernel as a formal Memory Resource Manager per ADR-025.
*   **Architecture Version:** V1 architecture stabilized (ADR-021/024 ratified)
*   **Architecture Status:** ADR-011 through ADR-024 ratified.

---

## Engineering Principles

1.  **Hardware Before Abstraction:** New architectural concepts must be designed to solve validated hardware realities. We test on physical targets first, then generalize.
2.  **Prototype Before RFC:** A new RFC requires a working experiment or a reference implementation to prove viability before it can be accepted.
3.  **Mechanism Before Policy:** The kernel implements strict, low-level execution mechanisms (e.g., page tables, hardware capability checks). Resource orchestration and placement policies live at a higher layer.
4.  **One Active Milestone:** Only one milestone is active at any point. Parallel research on future tracks is encouraged, but must not distract from the active release goal.
5.  **No Feature Debt:** Do not write new features if the active milestone's exit criteria are unmet or if there are outstanding warnings/clippy issues.

---

## Out of Scope (Temporarily Postponed)

The following topics are intentionally out of scope for the near and mid-term:
- Symmetric Multiprocessing (SMP)
- Non-Uniform Memory Access (NUMA) balancing
- Physical CXL/HBM/GPU device programming
- Hardened security modules / TrustZone
- Filesystems & Network stacks
- General-purpose userspace POSIX translation layers

---

## Dependency Graph

### Current Track — V-epics

```
P0  Rename + cleanup (TheseusOS → Vivanta) ✅
    ↓
P1  SystemState + Volatile Identity ✅ (V1.1)
    ↓
[M4.5.2 — RK3568 hardware stabilization] ✅ (included in V1.1)
    ↓
P2  V2/M5 Memory Resource Manager (integrate existing MemoryObject) ← ACTIVE
    ↓
P2  V2/M6 Resource-backed Runtime (first isolated EL0 process)
    ↓
P3  V3 Device Graph + Minimal Driver Contract (ADR-022)
    ↓
P3.5  Storage driver (SPI NAND / eMMC)
    ↓
P4  V4 Kernel Task model (spawn/exit/yield)
    ↓
P5  V5 Persistent Identity + Storage → closes ADR-024
    ↓
P5  V5 Service Framework (LoggingService first)
    ↓
P6  V6 Recovery Manager
    ↓
P7  V7 Additional hardware targets (RPi3, SDM660)
```

### Historical — Phases R2–R7 (superseded by V-epics)

```

Phase R2 Reality Lock ──► ✅ Completed via ACS, M3-AB, M4, M4.4, M4.5

Phase R3 Kernel Foundation ──► K1 (Interrupts) ✅, K2 (Scheduler) ✅
                                  K3–K5 deferred to V-epic model

Phase R4 Resource System ──► Folded into V2/M5 (MemoryObject integration)

Phase R5 Runtime ──► Deferred post-V5

Phase R6 Native Runtime ──► Deferred post-V6

Phase R7 Desktop/Mobile ──► Deferred post-V7
```

---

## Phase R2 — Reality Lock (✅ COMPLETED)

**Goal:** Transition from research prototype to engineering platform.

R2 was completed via the following milestones, redirected from the original lavender/SDM660 target to QEMU + RK3568:

- **ACS** (Architecture Cleanup Sprint) — kernel/arch/platform/target split, `extern "Rust"` boundary
- **M3-AB** — AArch64 + ARMv7 boot on QEMU with shared BootInfo contract
- **M3-BC** — MemoryObject lifecycle validated on QEMU
- **M4** — Cooperative multi-threading, scheduler, IRQ, timer, thread lifecycle
- **M4.4** — Address spaces, multi-AS model with hardware isolation
- **M4.4.5** — Unified context switch (ADR-017)
- **M4.5.0** — EL0 transition preparation
- **M4.5.1** — First EL0 entry and SVC roundtrip on QEMU
- **RK3568 Stage 1** — UART output, EL2→EL1 transition, Rust entry on hardware

The original lavender/SDM660 specific targets (M1-B0 through M1-B4) were superseded by RK3568 as primary hardware validation target.

---

## Phase R3 — Kernel Foundation (PARTIALLY COMPLETED)

**Goal:** Build execution foundation over early boot code.

### K1 — Interrupt Subsystem ✅ COMPLETED (M4)
- Exception vector tables, GICv3, ARM generic timer, timer IRQ.

### K2 — Scheduler ✅ COMPLETED (M4)
- Single-core round-robin, cooperative switching. True preemption deferred to physical hardware validation (RK3568).

### K3–K5 — DEFERRED to V-epic model
- Capability IPC, VMM integration, Capability Manager are folded into V2/M5 (Memory Resource Manager) and V4 (Task abstraction).
- VMM foundations (address spaces) completed in M4.4.

---

## Phase R4 — Resource System

**Goal:** Implement the `ResourceObject` hierarchy.

-   `ResourceObject` becomes the common abstraction for memory, devices, storage, and future resources.
-   MemoryObject becomes a specific subclass of `ResourceObject`.
-   Introduce `DeviceObject` (MMIO/DMA ranges) and `StorageObject` (blocks/flash).
-   **Exit Criteria:** Devices are accessed strictly through capability-verified resource handles.

---

## Phase R5 — Runtime

**Goal:** Jump to userspace execution.

-   Implement `init` process, user space runtime library, and early continuity daemon.
-   **Exit Criteria:** A user process successfully executes an IPC transaction with a kernel-managed ResourceObject.

---

## Phase R6 — Native Theseus Runtime

**Goal:** Implement placement and migration.

-   Active `MemoryPlacementPolicy` engines.
-   Automatic background migration of VMOs between tiers (e.g., DDR and simulated persistent memory/CXL).
-   **Exit Criteria:** Live migration of active pages without process interruption.

---

## Phase R7 — Desktop/Mobile

**Goal:** High-level OS subsystems.

-   Add file system, networking, and early graphical framebuffer drivers.

---

## Engineering Tracks

### Track A: Architecture
-   Maintain RFCs under `specs/rfc/` and ADRs under `docs/adr/`.
-   Strict rule: speculative RFCs are frozen. RFC status must remain `Experimental` until proven on target.

### Track B: Hardware Support
-   Current targets:
    `QEMU AArch64` (validation baseline) ──► `RK3568` (primary hardware target)
-   Future expansion (post V3 Device Graph):
    `Raspberry Pi 3B+` ──► `Qualcomm SDM660 (lavender)` ──► `x86_64` / `RISC-V`
-   RK3568 is a **validation target**, not the architecture foundation. Vivanta must not become "RK3568 OS". If RK3568 bring-up is blocked by hardware issues, architecture work continues on QEMU.

### Track C: Tooling & Automation
-   Build: `./build.sh rk3568`, `./build.sh qemu-aarch64` — per-target
-   Future: `cargo xtask` for standardised tasks (rename migration, flash, inspect-state)

### Track D: Verification
-   Maintain Unit tests.
-   Add QEMU-based continuous integration for PRs.
-   Acknowledge that manual hardware testing on the target device is required before closing milestones.

### Track E: Documentation
-   Keep `PROJECT_STATE.md` synchronized.
-   Keep this roadmap (`master-roadmap.md`) synchronized.
-   Keep RFC and ADR status up to date.
-   Always document architectural decisions before starting implementation.

---

## M1.0 — Early MMU Foundation (✅ COMPLETED 2026-07-27)

**Goal:** Establish minimal MMU infrastructure for all AArch64 targets — static identity map,
MAIR/TCR/TTBR0 configuration, MMU enable sequence, and memory discovery.

| Sub-step | Description | Status |
|----------|-------------|--------|
| M1.0.1 | Translation Tables — static L1+L2 page tables, descriptor constants | ✅ |
| M1.0.2 | Identity Mapping — 4 GB, 2 MB blocks, Normal WBWA | ✅ |
| M1.0.3 | MMU Enable — MAIR, TCR, TTBR0, SCTLR_EL1.M | ✅ |
| M1.0.4 | Validation — println! before/after MMU, all targets build | ✅ |
| M1.0.5 | Memory Discovery — subtract kernel/DTB/tables from BootInfo usable regions | ✅ |

UART refactoring (shared `Console`/`PL011`/`println!`, `platform-rpi3b`, GPIO init) was completed
as a prerequisite for M1.0.

**New files:**
- `arch-aarch64/src/early_mmu.rs` — static identity map, no allocator required
- `boot_common/src/pl011.rs` — shared PL011 driver
- `boot_common/src/memory_discovery.rs` — BootInfo → available regions
- `platform-rpi3b/` — GPIO init for RPi3 UART

## M1.1 — Physical Memory Manager (✅ COMPLETED 2026-07-27)

**Goal:** Self-contained bitmap PMM decoupled from BootInfo, with self-test and statistics.

| Sub-step | Description | Status |
|----------|-------------|--------|
| M1.1.1 | Bootstrap Bitmap — place bitmap in first AvailableRegion | ✅ |
| M1.1.2 | Page Allocation — allocate/free/reserve single pages, is_allocated | ✅ |
| M1.1.3 | Region Reservation — reserve_range(start, end) | ✅ |
| M1.1.4 | Self Tests — allocate→free→re-allocate smoke test in boot seq | ✅ |
| M1.1.5 | Statistics — total/reserved/allocated/free counters | ✅ |

**Contract:**
```
MemoryMap → discover() → AvailableRegion[] → PmmBitmap::new(region)
```
PMM has zero knowledge of BootInfo, DTB, or FDT.

## V-Epic Roadmap (Active Track)

V-epics replace the earlier R-phase model as the primary planning structure. M-numbers (M1–M5) are preserved alongside V-epics for historical continuity.

### P0 — Rename and Cleanup

| Sub-step | Description | Exit Criteria |
|----------|-------------|---------------|
| V0.1a | Cargo package rename (scripted, git tags) | `cargo check --workspace` + 2 build targets pass |
| V0.1b | Documentation rename | Build clean |
| V0.1c | Directory rename (`Vivanta/` → `Vivanta/`) | Full rebuild clean |
| V0.2 | Roadmap refresh (this document + PROJECT_STATE.md) | All references consistent |
| V0.3 | `docs/vivanta/` scaffold | Stubs created, no dead links |

**ADRs:** — (rename is execution, not architecture)

### P1 — System State and Identity ✅ (V1.1 completed 2026-07-23)

| Sub-step | Description | Exit Criteria | Status |
|----------|-------------|---------------|--------|
| V1.1 | `SystemState::from_boot_info()` + ownership structure per ADR-020 | SystemState constructed, BootInfo references escaped | ✅ Complete |
| V1.2a | Volatile IdentityState per ADR-023 | Identity generated per boot, enum match enforced | ✅ Complete (as Runtime Identity) |
| V1.2b | Persistent Identity (blocked on storage driver P3.5) | — | ⏳ Blocked |

**ADRs:** ADR-020, ADR-021, ADR-023, ADR-024

### M4.5.2 — RK3568 Hardware Stabilisation

| Objective | Fallback |
|-----------|----------|
| Boot kernel, receive BootInfo, execute diagnostic | If blocked by undocumented HW issue / missing firmware / board failure → continue architecture work on QEMU |
| Goal: `println!` + DTB on RK3568 | Hardware validation remains pending. RK3568 is **validation target**, not foundation. |

### P2 — V2 / M5: Memory Resource Manager

| Sub-step | Description | Exit Criteria | Status |
|----------|-------------|---------------|--------|
| V2.0 | ADR-011 pre-flight: frozen component adaptation review | Change documented, regression pass on QEMU | ✅ |
| V2.1 | ADR-025 MRM Integration Design — architecture decision ratified | ADR-025 proposed 2026-07-24 | ✅ |
| V2.2 | PmmBackend + MRM in SystemState (merge BootMemoryManager) | QEMU boots, MRM stats printed | ✅ |
| V2.3 | Runtime page table writer (arch-api + arch-aarch64) | MemoryObject::map() programs MMU | ✅ |
| V2.4 | QEMU smoke test — allocate, map, write, read, unmap | MemoryObject lifecycle validated on QEMU | ⏳ |
| V2.5 | PlacementPolicy formalization (Kernel, Device, Persistent, Fast) | Enum defined, wired to MRM | ✅ (policy.rs ported from frozen) |

**ADRs:** ADR-011 (Amendment: Frozen Component Unfreezing), ADR-025

### P2 — V2 / M6: Resource-backed Runtime

The first genuine isolated EL0 process. Not just a jump — a runtime environment.

| Sub-step | Description | Exit Criteria |
|----------|-------------|---------------|
| V2.6 | MemoryObject smoke test + L2 block splitting | Allocate→map→write→read→unmap via MRM+MMU |
| V2.7 | KernelHeap as MRM consumer (not PMM bypass) | Heap backed by MemoryObject |
| V2.8 | First EL0 process with allocated MemoryObjects | EL0 binary blob with MRM-allocated stack/heap |

**Goal:** `Task { address_space, memory_objects, identity, capabilities }` — a real process, not just a thread.

### P3 — V3: Device Graph and Driver Model

| Sub-step | Description | ADRs |
|----------|-------------|------|
| V3.0 | DeviceDescriptor metadata (not Capability — name reserved) | ADR-022 |
| V3.1 | Device Graph (devices/topology/capabilities only, NO driver state) | ADR-020 |
| V3.2 | Driver lifecycle contract: `trait Driver { fn init(); fn shutdown(); }` | ADR-022 |
| P3.5 | Storage driver (SPI NAND primary, eMMC secondary) | — |
| V3.3 | DriverManager (owns driver instances, not SystemState) | ADR-020 |

### P4 — V4: Execution Model Expansion

| Sub-step | Description |
|----------|-------------|
| V4.1 | Kernel Task object (Thread + Address Space + Resources) |
| V4.2 | Scheduler policy abstraction (concrete RoundRobin, deferred: priority/realtime) |

### P5 — V5: Service Framework

| Sub-step | Description |
|----------|-------------|
| V5.0 | Service architecture definition (kernel-service boundary, IPC/communication) |
| V5.1 | First service: **LoggingService** (UART, always needed, validates service boundary) |
| Future | DownloadService (post-R7 — requires networking + storage; aria2 as replaceable backend) |

### P6 — V6: Recovery Manager

| Sub-step | Description |
|----------|-------------|
| V6.0 | Recovery architecture definition (detect/malfunction → restore → reboot) |
| V6.1 | Volatile recovery prototype (QEMU, no storage dependency) |
| V6.2 | Full recovery with persistent identity (blocked on V1.2b) |

### P7 — V7: Additional Hardware Targets

| Target | Timing |
|--------|--------|
| QEMU AArch64 | ✅ Baseline (always active) |
| RK3568 | ✅ Active (primary) |
| Raspberry Pi 3B+ | Post-V3 (Device Graph + Driver model) |
| Qualcomm SDM660 (lavender) | Post-V3 |
| x86_64, RISC-V | Post-V7 |

---

## Design Watchlist (Architectural Lessons)

### Vivanta (Academic Research)
-   **Study:** Safe live component replacement, Rust typing as a safety boundary, module loading without ELFs.
-   **Do Not Copy:** Process-less monolithic single-address-space design, custom non-standard object format.

### seL4
-   **Study:** Capability derivation tree, authoritative revocation propagation model, formal safety bounds.
-   **Do Not Copy:** Pure microkernel decomposition where memory management is completely externalized to user space (too complex for early bringup).

### Zircon (Fuchsia)
-   **Study:** Virtual Memory Objects (VMO) structure, thread/process handle semantics.
-   **Do Not Copy:** Massive object-oriented C++ kernel codebase (maintain Rust-idiomatic functional approach).

### Barrelfish
-   **Study:** Heterogeneous systems support, hardware topology representation, distributed kernel concepts.
-   **Do Not Copy:** Purely distributed multi-kernel model where message-passing is mandatory even for local intra-SoC memory allocation.

### Redox
-   **Study:** Rust-first kernel design, scheme namespaces.
-   **Do Not Copy:** Unix/POSIX-first compatibility as a primary structural driver.

### Linux
-   **Study:** Memory allocators (buddy/slab), DMA API constraints.
-   **Do Not Copy:** Heavy global Virtual File System (VFS) and POSIX-first subsystem assumptions.

---

## Long-term Vision (R8+)

*The following goals represent long-term research directions rather than committed implementation milestones.*

-   **Autonomous Cryptographic Continuity:** The system automatically relocates itself to nearby available hardware when a physical machine is about to fail.
-   **Self-Evaluating Resource Topology:** The operating system continuously measures hardware parameters (coherence, power, latency) and adapts its own memory/execution layout dynamically.
-   **Hardware Independence:** The operating system shall preserve user identity, state, and execution semantics across fundamentally different processor architectures (e.g., transparently bridging ARM64 and RISC-V).

---

## Changelog

*Every roadmap modification must update this section.*

### 2026-07-12
-   Initial release of the Master Roadmap.
-   R2.0 and R2.1 marked as active focus.

### 2026-07-19
-   **Roadmap v1.1:** Replaced R-phase structure with V-epics (V0–V8).
-   Current Focus updated to M4.5.x/RK3568 + V0 rename.
-   Added V-epic dependency graph; R2 marked ✅, R3 K1/K2 ✅, K3–K5 deferred.
-   Hardware target list updated (lavender → RK3568).
-   RK3568 fallback policy documented.
-   V2.1 MemoryObject redefined as "hardware adaptation" (not redesign).
-   V5 restructured: LoggingService first, DownloadService deferred.
-   References new ADRs 020–023.

### 2026-07-23
-   **V1.1 completed:** Runtime Identity, ADR-021/024 ratified.
-   RK3568 boot flow unified with QEMU (adapter_main → FDT → BootInfo → kernel_main).
-   Current Focus updated to P2 V2/M5 Memory Resource Manager.
-   ADR-024 added to references.

### 2026-07-24
-   **V2/M5 Phases A–E completed:** PmmBackend created, MRM integrated into SystemState, BootMemoryManager removed.
-   ADR-025 (Memory Resource Manager Integration) proposed — design note with 5 implementation phases.
-   Runtime page table writer added: `mmu_map_object` / `mmu_unmap` in arch-api + aarch64 impl + test-stub stubs.
-   `MemoryObject::map()` now programs real MMU via arch-api, not just software-recorded slots.
-   `kernel/src/memory/` module created: 6 files (resource, capability, object, policy, manager, pmm_backend).
-   Full workspace build passes on QEMU AArch64, RK3568, test-stub, arch-aarch64.
-   V2/M5 substeps V2.0–V2.3 and V2.5 marked complete; V2.4 (QEMU smoke test) pending.
