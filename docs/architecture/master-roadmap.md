# Master Roadmap

**Purpose:** This is the single authoritative engineering document defining the development and porting sequence for TheseusOS over the next 1–2 years. It does not introduce new specifications, but maps existing architectural components (BootInfo, PMM, VMM, MemoryObject, Capability) onto a phased execution plan.

**Authority:** If any implementation plan, TODO list, sprint document, or experimental branch conflicts with this roadmap, this document takes precedence unless explicitly superseded by a newer approved version.

---

## Current Focus

*   **Active Milestone:** `M1-B0: First Light`
*   **Current Engineering Objective:** Obtain early UART output on the physical Xiaomi Redmi Note 7 (lavender / SDM660) target.
*   **Architecture Version:** R2
*   **Architecture Status:** Frozen (except for hardware-driven bugs and minor integration adjustments).
*   **Specification Status:** New RFC creation is suspended until `M1-B0` is successfully achieved.

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

```
Reality Lock (M1-B Milestones)
   M1-B0 (First Light / UART)
      │
      ▼
   M1-B1 (Hardware Discovery / DTB)
      │
      ▼
   M1-B2 (Minimal Kernel / MMU)
      │
      ▼
   M1-B3 (MemoryObject on HW)
      │
      ▼
   M1-B4 (Hardware Continuity Proof) ──► [Phase R2 Complete]

==================================================================

Kernel Phases (R3-R7)
   Phase R3: Kernel Foundation
      ├── Track K1 (Interrupts)
      ├── Track K2 (Round-Robin Scheduler)
      ├── Track K3 (Capability IPC)
      ├── Track K4 (Virtual Memory Manager)
      └── Track K5 (Capability Manager)
      │
      ▼
   Phase R4: Resource System (ResourceObject model)
      │
      ▼
   Phase R5: Runtime (Userspace init)
      │
      ▼
   Phase R6: Native Theseus Runtime (Placement Policies)
      │
      ▼
   Phase R7: Desktop/Mobile Integration
```

---

## Phase R2 — Reality Lock (CURRENT)

**Goal:** Transition TheseusOS from simulation (QEMU) onto physical hardware.

### M1-B0 — First Light
-   **Description:** Minimal boot loader/stub execution on Xiaomi Redmi Note 7.
-   **Tasks:** Initialize SDM660 PL011 UART, write a static identifier.
-   **Deliverable:** Raw serial output: `Theseus Boot v0.1 | Arch: ARM64 | SoC: SDM660`

### M1-B1 — Hardware Discovery
-   **Description:** Platform analysis via FDT/DTB.
-   **Tasks:** Scan DTB for memory map, CPU nodes, UART/GIC physical base addresses.
-   **Deliverable:** Verifiable Hardware Inventory printed over UART.

### M1-B2 — Minimal Kernel
-   **Description:** Early memory management.
-   **Tasks:** Set up AArch64 PageTableBuilder, enable MMU with direct maps, initialize PMM.
-   **Deliverable:** Safe execution inside virtual memory space.

### M1-B3 — MemoryObject on Hardware
-   **Description:** Validate the resource-oriented memory abstraction on a physical device.
-   **Tasks:** Run the full `MemoryObject` lifecycle (Create, Allocate, Map, Share, Revoke) on hardware.
-   **Deliverable:** Execution trace confirming zero raw physical allocations in kernel logic.

### M1-B4 — Hardware Continuity Proof
-   **Description:** Transfer M1-A continuity proof to physical hardware.
-   **Tasks:** Run the sequence: Genesis (generate keypair) → Normal boot (verify State Document) → Storage death simulation → Recovery via Seed.
-   **Deliverable:** Successful cryptographic recovery and boot mode transition on device.

### Phase R2 Exit Criteria
-   [ ] Raw UART output operates reliably on SDM660.
-   [ ] Device Tree parsed correctly into a standard memory map.
-   [ ] MMU activated successfully using the `PageTableGuard`.
-   [ ] PMM and heap are operational.
-   [ ] MemoryObject lifecycle validated.
-   [ ] Identity continuity restored successfully after simulated storage erasure.

---

## Phase R3 — Kernel Foundation

**Goal:** Build a minimal microkernel-style foundation over the early boot code.

### Track K1 — Interrupt Subsystem
-   Set up exception vector tables for ARM64/AArch32.
-   Implement early GICv2/v3 interrupt dispatcher.
-   Set up timer interrupts (ARM generic timer).

### Track K2 — Scheduler
-   Implement a minimal single-core Round-Robin thread scheduler.
-   No priorities, no affinity groups, no SMP.

### Track K3 — IPC
-   Introduce capability-aware IPC endpoints.
-   Strict synchronous message passing between early kernel threads.

### Track K4 — Virtual Memory Manager (VMM)
-   Integrate `MemoryObject` with the Virtual Memory Manager (VMM).
-   Add `VirtualMemoryObject` (VMO) mapping logic and `AddressSpace` management.

### Track K5 — Capability Manager
-   Transition `MemoryCapability` to a unified `Capability` model representing memory, devices, and IPC handles.

### Phase R3 Exit Criteria
-   [ ] Multi-threading operational with timer-driven context switching.
-   [ ] Synchronous IPC verified between isolated threads.
-   [ ] Page faults caught and routed through a default handler.

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
-   Gradual expansion of physical hardware targets:
    `QEMU (virt)` ──► `Redmi Note 7 (lavender)` ──► `Raspberry Pi 4` ──► `x86_64 PC`

### Track C: Tooling & Automation
-   Introduce standard tasks via `cargo xtask`:
    -   `cargo xtask build-qemu`
    -   `cargo xtask flash-lavender`
    -   `cargo xtask inspect-state`

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

## Design Watchlist (Architectural Lessons)

### Theseus OS (Another)
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
