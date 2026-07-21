# Independent Architecture Audit — Vivanta Roadmap

> **Audience**: External LLM architecture reviewers (ChatGPT, Gemini, Grok, DeepSeek).
> **Purpose**: Independent evaluation of the Vivanta roadmap and architectural decisions.
> **Output format**: Specified at the end of this document.

---

## 1. Project Context

**Vivanta** (formerly TheseusOS) is an operating system research and engineering project.

Vivanta is **not** a Linux clone, **not** a microkernel experiment, and **not** intended to reproduce Unix semantics.

The long-term objective is to create an operating system whose architecture prioritizes:

- **Continuity of system identity** — the system proves it is the same entity across hardware changes through a verifiable chain of signed State Documents.
- **Recoverability** — recovery is a design goal from the beginning, not an afterthought.
- **Hardware abstraction** — the kernel must not know the ISA or the board.
- **Component evolution** — components can be replaced without rebuilding the system.
- **Explicit architecture** — every architectural decision is documented before implementation.

The project intentionally prefers long-term architectural consistency over short-term implementation speed.

### 1.1 Vision Statement (from PROJECT_STATE.md)

> Vivanta is an operating system that preserves its identity and user environment across complete replacement of its hardware components. The central innovation is not a new kernel or driver model, but a formal protocol for system identity persistence across physical hardware transitions.

### 1.2 Core Invariants

These are fundamental, non-negotiable properties:

1. **User Environment Preservation** — user's applications, data, and settings persist across hardware changes.
2. **Hardware Adaptability** — the OS adapts to hardware; hardware does not dictate OS limitations.
3. **Architecture Independence** — the core platform must be portable across major architectures (x86_64, ARM, RISC-V).
4. **Minimal Friction** — system automates decisions; reduces manual user configuration.
5. **Documentation as Source of Truth** — architecture is documented before implementation.
6. **Long-Term Maintainability** — architecture must be designed for evolution over decades.
7. **Identity Independence** — the Root Keypair must not depend on the component it is designed to survive replacement of.
8. **No Booting in Unknown State** — if identity cannot be resolved, the system halts rather than booting in an indeterminate state.

---

## 2. Current Status

### 2.1 Completed Milestones

| Milestone | Status | Summary |
|-----------|--------|---------|
| R0 | ✅ Complete | Peer review, architecture audit, identity resolved |
| RFC Chain | ✅ Complete | RFC-001 through RFC-010 |
| M1-A | ✅ Complete | Continuity Proof Experiment on QEMU — core thesis validated |
| M2-A | ✅ Complete | Environment Continuity Experiment — user data persistence validated |
| M3-A | ✅ Complete | Incremental Environment Continuity |
| M3-B | ✅ Complete | Memory Object Foundation — resource-oriented memory model (QEMU) |
| M3-C | ✅ Complete | Memory Object Semantics — lifecycle, clone, share, revoke |
| M3-AB | ✅ Complete | AArch64 + ARMv7 boot on QEMU with shared BootInfo contract |
| R2 | ✅ Complete | Reality Lock — architecture freeze, repository reorg |
| ACS | ✅ Complete | Architecture Cleanup Sprint — kernel/arch/platform/target split, `extern "Rust"` contract |
| M4 | ✅ Complete | Execution Foundation — cooperative multi-threading, timer, thread lifecycle |
| M4.4 | ✅ Complete | Address Spaces — multi-AS model with verified hardware isolation |
| M4.4.5 | ✅ Complete | Execution Contract Freeze — unified context switch (ADR-017) |
| M4.5.0 | ✅ Complete | EL0 Transition Preparation — `InterruptGuard`, `eret_to_user_stub` |
| M4.5.1 | ✅ Complete | First EL0 entry + SVC roundtrip on QEMU |

### 2.2 In Progress

| Milestone | Status | Summary |
|-----------|--------|---------|
| M4.5.2 | 🔧 In Progress | RK3568 bring-up: `println!` + DTB on real hardware. Diagnostic compiled, blocked on hardware connection. |

### 2.3 Deferred

| Item | Reason |
|------|--------|
| M1-B (lavender/SDM660 hardware bringup) | Superseded by RK3568 as primary hardware target |
| True preemption on QEMU | Blocked: writing to on-stack ExceptionFrame from IRQ prevents subsequent timer IRQs. Deferred to physical ARM64 hardware. |

### 2.4 Current Development Targets

- **RK3568** (Cortex-A55, 4 GiB DDR4, NS16550 UART, SPI NAND) — primary hardware target
- **QEMU AArch64 virt** — architecture validation baseline

### 2.5 Future Hardware Targets

- Raspberry Pi 3B+
- Qualcomm SDM660 (lavender)
- Additional ARM platforms

---

## 3. Validated Architectural Claims

The following claims have been experimentally validated:

| # | Claim | Validated In |
|---|-------|-------------|
| 1 | Ed25519 keypair generation, signing, and verification | M1-A |
| 2 | BIP-39 seed → deterministic keypair derivation | M1-A |
| 3 | State Document creation, signing, signature verification | M1-A |
| 4 | State Chain linkage and chain verification | M1-A |
| 5 | Full recovery flow: seed → keypair → continuity proof | M1-A |
| 6 | Three boot modes (Genesis, Normal, Recovery) as a state machine | M1-A |
| 7 | Identity independence from storage | M1-A |
| 8 | Environment Manifest creation, signing, verification | M2-A |
| 9 | User data integrity hashing and verification | M2-A |
| 10 | Storage replacement with identity + environment preserved | M2-A |
| 11 | Environment chain independent of state chain | M3-A |
| 12 | Incremental environment updates without state migration | M3-A |
| 13 | Cross-link consistency between State Documents and Environment Manifests | M3-A |
| 14 | AArch64 + ARMv7 boot on QEMU with shared BootInfo contract | M3-AB |
| 15 | Page table construction and MMU enable on two architectures | M3-AB |
| 16 | MemoryObject: create, allocate, map, clone, share, revoke lifecycle | M3-C |
| 17 | Resource-oriented memory model: MemoryBackend, MRM, placement policy | M3-BC |
| 18 | Multiple virtual mappings per MemoryObject | M3-C |
| 19 | Kernel depends only on arch-api (not arch-aarch64) | ACS |
| 20 | Thread scheduling policy in kernel, context switching mechanism in arch | ACS |
| 21 | Through-type boundary: `ArchContext = usize`, `InterruptFrameHandle = usize` | ACS |
| 22 | `extern "Rust"` bidirectional contract between kernel and arch | ACS |
| 23 | MMIO addresses moved from kernel to platform (BootInfo.mmio_regions) | ACS |
| 24 | Build-time proof: kernel links with arch-test-stub (no real ISA) | ACS |
| 25 | boot-info crate: zero dependencies, core-only contract types | ACS |

---

## 4. Codebase Structure

### 4.1 Workspace Layout

```text
vivanta-boot/                         (workspace root)
├── Cargo.toml                        (workspace members)
├── build.sh                          (per-target build driver)
├── linker.ld
│
├── boot-info/                        Zero-dep, core-only contract types
├── boot_common/                      Console, println!, NS16550, FDT, BootContext
├── arch-api/                         extern "Rust" trait contracts (no ISA)
├── arch-aarch64/                     AArch64 implementation
├── arch-armv7a/                      ARMv7-A implementation (frozen)
├── arch-test-stub/                   No-ISA stub for build-time proof
│
├── kernel/                           Architecture-independent kernel logic
├── kernel-memory-frozen/             Frozen RFC prototypes (ADR-011)
│
├── platform-qemu/                    QEMU virt platform support
├── platform-rk3568/                  Rockchip RK3568 platform support
├── platform-sdm660/                  Qualcomm SDM660 (lavender)
├── platform-allwinner-h616/          Allwinner H616
│
├── target-qemu-aarch64/              Final binary: QEMU AArch64
├── target-qemu-armv7a/               Final binary: QEMU ARMv7
├── target-rk3568/                    Final binary: RK3568
├── target-x96q/                      Final binary: X96Q (Allwinner H616)
├── target-lavender/                  Final binary: Redmi Note 7
├── target-test/                      Final binary: kernel + arch-test-stub
│
└── archive/boot_legacy/              Pre-ACS adapters (frozen, not active)
```

### 4.2 Dependency Direction (ADR-014)

```text
Target (final binary composition)
  ├── Platform (board/SoC)
  ├── Kernel (architecture-independent logic)
  └── Arch implementation (ISA-specific)

Kernel → arch-api contracts only
Arch implementation → arch-api contracts only
Platform → boot-info
```

### 4.3 Forbidden Dependencies

```text
kernel → arch-aarch64          (kernel must not know the ISA)
kernel → platform-*            (kernel must not know the board)
arch   → platform-*            (arch must not know the SoC)
platform → kernel              (platform is pre-kernel)
platform → arch                (platform describes hardware, doesn't drive it)
```

### 4.4 Arch Boundary Contract (ADR-015)

`arch-api` declares function signatures in `extern "Rust"` blocks. Each arch implementation provides `#[no_mangle]` functions.

```rust
// arch-api/src/context.rs
pub mod context {
    extern "Rust" {
        pub fn switch_context(old: *mut usize, new: usize);
    }
}

// arch-aarch64/src/context.rs
#[no_mangle]
pub fn switch_context(old: *mut usize, new: usize) { /* AArch64 impl */ }
```

- No vtable, no runtime dispatch, no trait boilerplate.
- Only ONE arch implementation crate is linked per target binary.
- `extern "Rust"` (not `extern "C"`) allows LTO across the boundary.

---

## 5. Existing RFCs

| RFC | Title | Status |
|-----|-------|--------|
| RFC-001 | Identity Model | ✅ Accepted (validated M1-A) |
| RFC-001.5 | Identity Utility Model | ✅ Accepted |
| RFC-002 | Bootstrap Architecture | ✅ Accepted |
| RFC-003 | Boot Protocol | ✅ Accepted (superseded by RFC-008) |
| RFC-004 | Recovery Seed Format (BIP-39) | ✅ Accepted |
| RFC-005 | State Document Format | ✅ Accepted |
| RFC-006 | Environment Continuity Model | ✅ Accepted |
| RFC-007 | Dynamic Device Tree and Hardware Adaptation | 📝 Draft |
| RFC-008 | Boot Protocol (revised) | ✅ Accepted |
| RFC-009 | Platform Capability Model & BootInfo Contract | ✅ Accepted |
| RFC-010 | Memory Resource Model | 🔬 Experimental |

### 5.1 Key Identity Concepts (RFC-001/004/005/006)

```text
SystemIdentity     = Ed25519 keypair (the canonical identity)
Root Keypair       = genesis keypair from which all claims derive
Recovery Seed      = BIP-39 12-word mnemonic → regenerates Root Keypair
State Document     = signed CBOR document recording hardware/software inventory
State Chain        = ordered sequence of State Documents linked by hashes
Genesis State      = State Document 0, created at first boot
Environment Manifest = signed document recording user data hash, config hash, software inventory
Environment Chain  = ordered sequence of Environment Manifests
Continuity         = same Root Keypair + verified State Chain = same system
Fork               = two systems sharing a Root Keypair but diverging State Chains
Divorce Statement  = signed document establishing a fork as independent identity
```

### 5.2 Memory Resource Model (RFC-010, Experimental)

```text
AllocationRequirements (size, latency, bandwidth, persistence, policy)
    │
    ▼
Memory Resource Manager
    │
    ├── MemoryBackend: RAM    (latency=main,   persistence=volatile)
    ├── MemoryBackend: HBM    (latency=near,   persistence=volatile)   [future]
    ├── MemoryBackend: CXL    (latency=far,    persistence=persistent) [future]
    └── MemoryBackend: VRAM   (latency=near,   persistence=volatile)   [future]
```

**Note**: The full `MemoryObject` lifecycle (create/allocate/map/clone/share/revoke) was validated on QEMU in M3-BC, then frozen per ADR-011 until hardware validation prerequisites exist.

---

## 6. Existing ADRs

| ADR | Title | Status |
|-----|-------|--------|
| ADR-011 | Phase Transition — Research Prototype → Engineering Platform | Accepted |
| ADR-012 | Execution Model — ThreadContext vs ExceptionFrame | **Superseded by ADR-017** |
| ADR-013 | Privilege Transition Model — EL1 ↔ EL0 | Accepted |
| ADR-014 | Architectural Boundaries | Accepted |
| ADR-015 | Arch Boundary Contracts (`extern "Rust"`) | Accepted |
| ADR-016 | (referenced in roadmap discussion; not located in repository) | — |
| ADR-017 | Unified Execution Context | Accepted |
| ADR-018 | User Entry Transition Model | Proposed |
| ADR-019 | User Page Permissions and EL0 Memory Model | Proposed |

### 6.1 Key ADR-011 Principle

> **No abstraction before second implementation.**
>
> Before creating a trait, interface, or generic abstraction:
> 1. Two independent implementations must exist in the codebase
> 2. Common behavior must be demonstrable (not speculative)
> 3. The interface must reduce measurable duplication
> 4. The interface must be tested against both implementations
>
> Otherwise: keep local implementations. Do not abstract.

### 6.2 Key ADR-017 Decision

Unified context switch: a single `context_switch()` operates on `ThreadContext` (callee-saved registers + SP_EL1). The same function is used for both cooperative and (future) preemptive switching. `ExceptionFrame` is never copied between threads — it is a transient artefact of exception handling on the thread's own kernel stack.

---

## 7. Architectural Principles

The following principles are already considered stable.

### 7.1 Hardware First

Do not introduce abstractions before at least two hardware implementations require them. Test on physical targets first, then generalize.

### 7.2 Immutable Boot Contract

`BootInfo` is immutable. It exists only to transfer information from the bootloader into the kernel. It must never become global mutable runtime state.

### 7.3 Runtime Owns State

After boot, runtime state belongs to dedicated runtime objects rather than `BootInfo`. The kernel constructs a `SystemState` from `BootInfo` and then discards `BootInfo`.

### 7.4 Strict Layering

```text
Target
    ↓
Platform
    ↓
Kernel
    ↓
Architecture (via arch-api contracts)
```

The kernel must not directly depend on architecture-specific implementations.

### 7.5 Identity as First-Class Concept

Vivanta treats system identity as a first-class architectural concept. Identity is independent of:

- current hardware
- current storage
- current runtime instance

Persistent identity will be implemented only after storage support exists.

### 7.6 Recovery by Design

Recovery should rebuild runtime state while preserving system identity whenever possible. Recovery is not an afterthought.

### 7.7 Services Over Kernel Features

Kernel functionality should remain minimal. Whenever practical, new functionality should become a system service rather than additional kernel code.

```text
Application
    ↓
Download API
    ↓
Download Service
    ↓
Backend (aria2 / native engine)
```

### 7.8 Device Graph Discipline

Hardware discovery produces a Device Graph. Drivers consume the graph. The graph itself should describe only:

- devices
- topology
- capabilities

It must **not** own driver runtime state. Drivers reference the Graph; the Graph does not reference Drivers (no cyclic dependency).

### 7.9 Layered Memory Architecture

```text
Application
    ↓
MemoryObject
    ↓
Placement Policy
    ↓
Memory Resource Manager
    ↓
Virtual Memory Manager
    ↓
Physical Memory Manager
```

### 7.10 Mechanism Before Policy

The kernel implements strict, low-level execution mechanisms (page tables, hardware capability checks). Resource orchestration and placement policies live at a higher layer.

---

## 8. Known Constraints

- **Single primary developer** with AI-assisted engineering.
- **No SMP** (intentionally postponed).
- **No NUMA** (intentionally postponed).
- **No CXL/HBM/GPU device programming** (intentionally postponed).
- **No filesystem or network stack** (deferred to R7).
- **No POSIX translation layer** (explicitly out of scope).
- **No hardened security modules / TrustZone** (deferred past M1).
- Current priorities: architectural correctness, maintainability, incremental milestones, hardware validation.
- **Large rewrites without significant architectural benefit are discouraged.**

---

## 9. Current Assumptions

Unless demonstrated otherwise, assume:

- `BootInfo` is correct.
- Existing milestones work as documented.
- ACS was successfully completed.
- Current layering is intentional.
- The roadmap describes future work rather than the current implementation.
- The 25 validated architectural claims (Section 3) are accurate.

---

## 10. Architecture Decisions Already Locked

The following decisions are considered final unless a critical architectural flaw is found:

- Architecture Cleanup Sprint (ACS) — kernel/arch/platform/target split
- `BootInfo` contract (RFC-008/009)
- Layer boundaries (ADR-014)
- `extern "Rust"` arch boundary contract (ADR-015)
- Platform/Kernel separation
- M4 execution architecture (cooperative threads, timer, IRQ)
- Unified context switch (ADR-017)
- EL1↔EL0 privilege transition (ADR-013)
- "No abstraction before second implementation" (ADR-011)

**These decisions should not be revisited unless there is a demonstrable correctness issue.**

---

## 11. Scope of this Audit

Please review the roadmap **within this architectural philosophy**.

**Do NOT** recommend changes that intentionally move the project toward Linux, BSD, Windows, or a conventional microkernel unless you can clearly justify why such a change would improve Vivanta's stated goals.

**DO** challenge assumptions if they conflict with the project's own design principles or create engineering problems.

**Do NOT** recommend additional abstractions, components, or subsystems unless they solve a concrete architectural problem.

**AVOID** "future-proofing" that increases complexity without immediate benefit.

**DO** identify missing prerequisites, ordering errors, cyclic dependencies, and hidden coupling.

---

## 12. Audit Tasks

Please evaluate the roadmap in the following categories.

### 12.1 Architecture

Look for:

- architectural inconsistencies
- layering violations
- cyclic dependencies
- incorrect sequencing
- hidden coupling
- abstractions introduced too early (violation of ADR-011)
- violations of the "Device Graph knows no driver state" principle
- violations of the "BootInfo is immutable and transient" principle

### 12.2 Engineering Risk

Identify:

- risky rename operations (V0 touches ~21 Cargo.toml + ~20 .rs files)
- migration risks (package name vs crate name vs filesystem path — three distinct entities)
- rollback problems (the plan proposes atomic P0.1 → P0.5 sub-steps)
- integration risks (RK3568 println! diagnostic blocked on hardware)
- build-system issues (build.sh per-target, linker at 0x20500000)

### 12.3 Roadmap Ordering

Evaluate whether:

- dependencies are correct (e.g., V1.2b persistent identity blocked on storage driver — is this the right call?)
- work should be reordered
- milestones are missing
- prerequisites are incomplete
- the "parallel V-work during hardware wait" decision is sound

### 12.4 Kernel Design

Specifically review:

- `SystemState::from_boot_info()` — discarding BootInfo after construction
- `BootInfo` lifecycle (immutable, transient)
- Memory Resource Manager integration (V2/M5 uses **existing** MemoryObject, not new prototype)
- Device Graph (Device/Connections/Capabilities only — no driver state)
- Driver model (probe/init/shutdown — no hotplug yet)
- Storage dependency chain (Device Graph → Storage → Persistent Identity)
- Identity model (V1.2a volatile now, V1.2b persistent after storage)
- Recovery architecture (V6 depends on V1.2b)

Question assumptions wherever appropriate.

### 12.5 Long-term Maintainability

Evaluate whether the roadmap scales to:

- multiple architectures (x86_64, RISC-V additions)
- NUMA
- persistent memory
- CXL
- hotplug
- distributed services

If it does not scale, identify **where** the architecture will need to evolve, but do not propose premature abstractions.

---

## 13. Response Format

For every issue provide:

```text
Severity:    Critical / Major / Minor

Reasoning:   <why this is a problem, referencing specific roadmap items or principles>

Suggested Change:   <concrete, actionable>

Trade-offs:   <what this change costs or risks>
```

If you believe a section of the roadmap is already correct, explicitly explain **why** instead of proposing unnecessary changes.

**Avoid generic praise. Challenge every architectural assumption.**

---

## 14. Final Assessment

At the end of your review provide:

- **Overall architectural quality** (1–10)
- **Implementation readiness** (1–10)
- **Confidence level** (in your own assessment)
- **Top three architectural strengths**
- **Top three architectural risks**
- **Would you approve this roadmap for implementation in its current form? Why or why not?**

---

# Candidate Implementation Roadmap

## Overview

The roadmap is organized into **V-epics** (V0–V8) mapped to **priority levels** (P0–P7) and **milestone numbers** (M5+) where applicable. V-epics describe architectural intent; M-numbers preserve continuity with the completed M1–M4 lineage.

```text
P0  Rename + cleanup
    ↓
P1  System State + Volatile Identity
    ↓
[M4.5.2 — hardware stabilization, pauses V-work]
    ↓
P2  M5: Memory Resource Manager (integrate existing MemoryObject)
    ↓
P3  Device Graph + Driver trait
    ↓
P3.5  Storage driver (SPI NAND / eMMC)
    ↓
P4  Task abstraction + Scheduler policies
    ↓
P5  Service Layer + Download Service
    ↓
P6  Recovery Manager
    ↓
P7  Additional hardware targets (RPi3, SDM660)
```

---

## P0 — Rename and Cleanup

### V0.1a — Cargo Package Rename

**Atomic step. Build and verify before proceeding.**

- Rename `[package] name` in all ~21 `Cargo.toml` files: `boot-common` → `vivanta-boot-common`, `kernel` → `vivanta-kernel`, `arch-aarch64` → `vivanta-arch-aarch64`, etc.
- Update all `[dependencies]` references (package names only).
- Update workspace `members` list in root `Cargo.toml`.
- Update `use` statements in `.rs` files: `use boot_common::...` → `use vivanta_boot_common::...`
- **Three distinct entities** (must not be confused):
  - **Package name**: `boot-common` → `vivanta-boot-common`
  - **Rust crate name**: `boot_common` → `vivanta_boot_common` (hyphen→underscore)
  - **Filesystem path**: `../boot_common` → **UNCHANGED**
- Update `build.sh` package references.
- Update `description` fields: "Vivanta —" → "Vivanta —"

**Verify**:
- `cargo metadata` succeeds
- `cargo check --workspace` — 0 warnings
- `./build.sh rk3568` — 0 warnings
- `./build.sh qemu-aarch64` — 0 warnings

### V0.1b — Documentation Rename

- `README.md`, `PROJECT_STATE.md`, `MANIFESTO.md`: branding → Vivanta
- Source comments: branding → Vivanta
- **Preserved (do NOT change)**:
  - ADR titles (historical)
  - RFC numbers
  - "Ship of Theseus" philosophy term
  - Links to Vivanta research
  - Git history

**Verify**: `./build.sh rk3568` still clean.

### V0.1c — Directory Rename

- `Vivanta/` → `Vivanta/`
- `vivanta-boot/` → `vivanta-boot/`
- Inner crate directories (`boot_common/`, `kernel/`, etc.) remain unchanged.

**Verify**: full rebuild after rename.

### V0.2 — Roadmap Refresh

- `master-roadmap.md`:
  - Update "Current Focus" from M1-B0/lavender → M4.5.x/RK3568
  - Mark R2 Phase as ✅ (lavender survey done, RK3568 is the new hardware target)
  - Update Phase R3 to reflect M4 done (K1 interrupts ✅, K2 scheduler ✅)
  - Add V-epic structure (V0–V8) alongside existing R-phases
- `PROJECT_STATE.md`:
  - Update M4.4 status → ✅ Complete
  - Add M4.5.0, M4.5.1 → ✅ Complete
  - Add M4.5.2 → 🔧 In Progress
  - Add V-epic table

### V0.3 — Documentation Scaffold

Create `docs/vivanta/` structure:

```text
docs/vivanta/
├── architecture/
│   ├── overview.md
│   ├── boot.md
│   ├── memory.md
│   └── execution.md
└── philosophy/
    ├── identity.md
    └── continuity.md
```

**P0 Exit Criteria**:
- 0 warnings on all platforms
- `cargo metadata` correct
- No "Vivanta" branding remains (historical ADR/RFC references preserved)
- Roadmap and PROJECT_STATE reflect actual state

---

## P1 — System State and Volatile Identity

### V1.1 — Boot State Object

Create `kernel/src/state/mod.rs`:

```rust
pub struct SystemState {
    identity: SystemIdentity,
    hardware: HardwareInventory,
    runtime: RuntimeState,
}
```

- Constructor: `SystemState::from_boot_info(&BootInfo) -> Self`
- **BootInfo is NOT stored** — used only as constructor input, then discarded
- Wire into `kernel_main()`:
  ```rust
  pub unsafe fn kernel_main(info: &BootInfo) -> ! {
      let state = SystemState::from_boot_info(info);
      // BootInfo is no longer referenced after this point
      ...
  }
  ```

### V1.2a — Volatile SystemIdentity

Integrate existing RFC-001 spec into the real kernel boot path (previously only validated in QEMU experiment).

```rust
pub struct SystemIdentity {
    uuid: Uuid,
    keypair: Ed25519Keypair,
    genesis_timestamp: u64,
    capabilities: CapabilitySet,
}
```

- Generated fresh on each boot (volatile — lost on reboot)
- Integrated with `SystemState`

### V1.2b — Persistent SystemIdentity — **BLOCKED**

- Depends on: SPI NAND or eMMC storage driver (P3.5)
- Will persist UUID + keypair across reboots
- Deferred until storage is available

---

## M4.5.2 — RK3568 Hardware Stabilization (Pauses V-Work)

**Trigger**: board connection. When triggered, pause P1/V-work.

### Diagnostic (compiled, ready)

CPACR_EL1 + fmov test. Expected UART output: `KC{0-3}FG`

| Output | Interpretation | Next Step |
|--------|---------------|-----------|
| `KC0F` | FPEN=0, fmov traps | Fix CPACR_EL1 in EL2 path + CPTR_EL2.TFP |
| `KC3FG` | FPEN=3, fmov OK | Switch to `compare_exchange`/lock investigation in `GlobalConsole` |
| `KC0FG` | FPEN=0 but fmov OK | FP works despite CPACR — other cause |
| `KC3F` | FPEN=3 but fmov traps | Other trap cause (CPTR_EL2?) |

After fix: enable `FdtScanner::console()` + `build_memory_map()` from platform-rk3568.

---

## P2 — V2 / M5: Memory Resource Manager

**Key principle**: Integrate the **existing** `MemoryObject` (validated in M3-BC, frozen per ADR-011). Do **NOT** write a new prototype.

### V2.1 — MemoryObject Integration

- Unfreeze `kernel-memory-frozen/` prototypes
- Integrate `MemoryObject` lifecycle into the RK3568 kernel boot path
- Validate on real hardware (was M1-B3 goal)

### V2.2 — Placement Policy Formalization

```rust
pub enum PlacementPolicy {
    Kernel,
    Device,
    Persistent,
    Fast,
}
```

Future extensions (NUMA, RAM tiers, CXL, NVRAM) land here without changing the layers above.

---

## P3 — V3: Device Graph and Driver Component Model

### V3.1 — Device Graph

```text
Device Graph
├── CPU → GIC → Timer
├── Memory → MMU
└── UART → Console Driver
```

**Principle**: The Graph knows only:
- devices
- topology (connections)
- capabilities

It must **NOT** own driver runtime state or service state. Drivers reference the Graph; the Graph does not reference Drivers.

### V3.2 — Driver Trait

```rust
trait Driver {
    fn probe();
    fn init();
    fn shutdown();
}
```

No hotplug yet.

---

## P3.5 — Storage Driver (Unblocks V1.2b)

- SPI NAND (RK3568) or eMMC
- Depends on V3 (Driver Component Model)
- **Unblocks**: V1.2b Persistent SystemIdentity

---

## P4 — V4: Execution Model Expansion

### V4.1 — Kernel Task Object

```text
Task
 ├── Thread
 ├── Address Space
 └── Resources
```

### V4.2 — Scheduler Abstraction

```text
Scheduler
 ├── Policy
 │   └── RoundRobin (current)
 └── (future: priority, realtime, energy aware)
```

---

## P5 — V5: Service Layer

### V5.1 — Download Service (First Candidate)

```text
Application
    ↓
Download API
    ↓
Download Service
    ↓
Backend: aria2 / native engine
```

aria2 remains a replaceable backend.

---

## P6 — V6: Recovery System

```text
Broken system
    ↓
Recovery Environment
    ↓
Restore Identity
    ↓
Rebuild Runtime
```

**Depends on**: V1.2b (persistent identity)

`RecoveryManager` minimum:
- detect corrupted state
- validate manifest
- restore known-good configuration

---

## P7 — V7: Hardware Targets

| Target | Priority | Notes |
|--------|----------|-------|
| QEMU AArch64 | ✅ Baseline | Architecture validation |
| RK3568 | ✅ Active | Primary hardware target |
| Raspberry Pi 3B+ | P7 | ARMv8, accessible hardware, large user base |
| Nothing Phone 2 | Later | Qualcomm boot chain, closed firmware, GPU, PM — high risk |
| SDM660 (lavender) | P7 | Original M1-B target, deferred |

---

## Roadmap Dependency Graph (Final)

```text
P0.1 Cargo rename → build
   ↓
P0.2 Docs rename → build
   ↓
P0.3 Roadmap refresh
   ↓
P0.4 Docs scaffold
   ↓
P0.5 Directory rename → build
   ↓
P1.1 SystemState (from_boot_info, discards BootInfo)
   ↓
P1.2a Volatile Identity
   ↓
[M4.5.2 when board connects — pause V-work]
   ↓
M5 integrate existing MemoryObject
   ↓
V3 Device Graph (Device/Connections/Caps only)
   ↓
V3.2 Driver trait
   ↓
P3.5 Storage Driver
   ↓
V1.2b Persistent Identity
   ↓
V4 Task + Scheduler policies
   ↓
V5 Service Layer (Download Service)
   ↓
V6 Recovery Manager
   ↓
V7 Additional hardware targets
```

---

## End of Candidate Roadmap

---

# Audit Instructions (Reiterated)

You are now asked to evaluate the roadmap above according to the criteria in Section 12, using the response format in Section 13, and concluding with the Final Assessment in Section 14.

**Remember**:
- This is an architecture-first project, not a Linux clone.
- ADR-011's "no abstraction before second implementation" is a locked decision.
- The existing `MemoryObject` (M3-BC) is to be **integrated**, not reimplemented.
- `BootInfo` is immutable and transient — it must not become runtime state.
- The Device Graph must not own driver state.
- Persistent identity is intentionally blocked on storage driver availability.
- Single developer, AI-assisted — large rewrites are discouraged.

Challenge every assumption. Avoid generic praise. Provide concrete, actionable feedback.

---

# Appendix: Post-Audit Decisions (2026-07-19)

## Summary

Four independent audits (ChatGPT, DeepSeek, Gemini, Grok) reviewed the Vivanta roadmap. The following table consolidates every issue raised and the project's disposition.

| # | Theme | Issue | Raised By | Severity | Decision | Resolution |
|---|---|---|---|---|---|---|
| 1 | P0 Rename | Atomic rename risk — no rollback | All 4 | Critical→Major | **Accepted** | Scripted phased rename with git checkpoints. Three phases with `cargo check` + build after each. Rejected: cargo aliases (transient compatibility abstraction). |
| 2 | P0 Rename | String literals missing search | ChatGPT | Minor | **Accepted** | Automated grep for `boot-common`, `boot_common`, `Theseus`, `theseus`. |
| 3 | Identity | Semantic ambiguity: volatile vs persistent | ChatGPT, Grok | Major→Minor | **Accepted** | `IdentityState` enum per ADR-023. One variant per state (Volatile/Persistent). Monotonic transition. Rejected: full type hierarchy (premature), marker trait (over-engineered). |
| 4 | Identity | V1.2b blocks V6 Recovery | DeepSeek | Major | **Rejected** | Volatile recovery prototype rejected — recovery without persistence is testing infrastructure, not architecture. V6 remains blocked on storage driver. |
| 5 | SystemState | Ownership boundary undefined → God Object risk | ChatGPT, Grok | Major | **Accepted** | ADR-020: positive + negative ownership rules. SystemState owns coordination state only. No drivers, no raw resources, no service internals. HardwareState immutable. |
| 6 | BootInfo | Escape prevention — no validation | DeepSeek, Gemini, Grok | Major→Minor | **Accepted** | ADR-021: copy semantics from BootInfo, no `&'static` escapes, invariant documented. `kernel_main` signature unchanged (ADR-015 locked). |
| 7 | Driver trait | Premature abstraction (ADR-011 violation) | Gemini | Major | **Rejected** | Driver trait kept as lifecycle contract per ADR-022. Rationale: 3 existing entities (UART, GIC, Timer) share lifecycle. Capability methods explicitly excluded until 2 implementations require them. |
| 8 | Driver lifecycle | Underspecified — no state machine | ChatGPT | Major | **Accepted** | ADR-022: 5-state lifecycle documented (Discovered → Probed → Initialized → Running → Shutdown). Trait remains minimal (`init`, `shutdown`). |
| 9 | Device Graph | Capability definition missing | DeepSeek | Major | **Accepted** | `DeviceDescriptor` metadata (not `Capability` — name reserved). MmioRegion, InterruptLine, DmaRegion. Data, not abstraction. |
| 10 | Storage dependency | Deadlock in Device Graph → Storage chain | DeepSeek | Critical | **Rejected** | Validation path exists implicitly: UART + Timer already validate Device Graph. Storage is first complex consumer, not the first. |
| 11 | RK3568 fallback | No strategy if hardware fails | All 4 | Major | **Accepted** | Goal-based fallback: "boot kernel + receive BootInfo + execute M4 diagnostic." If blocked → continue on QEMU. No time limit. RK3568 is validation target, not foundation. |
| 12 | MemoryObject | Unfreezing mechanics undefined | DeepSeek | Critical | **Rejected** | Formal V2.0 review milestone rejected as excessive. ADR-011 amendment documents unfreezing criteria: hardware necessity, documented change, regression pass, integration not redesign. |
| 13 | Memory vs drivers | Policy expansion before storage | ChatGPT | Major | **Accepted** | PlacementPolicy kept as enum definition. Policy logic deferred until one real storage driver exists. |
| 14 | Download Service | Network dependency before R7 | Gemini | Critical | **Accepted** | V5 restructured: LoggingService first (UART, always needed, validates service boundary). DownloadService deferred to future reference. |
| 15 | Hardware targets | RPi3 too late — architecture lock-in risk | Gemini, DeepSeek | Major→Minor | **Rejected** | RPi3 stays at P7. Single developer cannot maintain 2 parallel physical targets. Architecture validation via QEMU + RK3568 is sufficient. RPi3 after V3 Device Graph. |
| 16 | Service framework | No definition of service vs kernel feature | DeepSeek | Minor | **Accepted** | V4.5 Service Framework milestone added: defines kernel-service boundary, communication protocol. |
| 17 | x86_64 / RISC-V | Not in roadmap | DeepSeek | Minor | **Accepted** | Added to P7 hardware targets as post-V7. Architecture claim is credible: `arch-api` + `extern "Rust"` boundary is ISA-neutral. |
| 18 | Driver state ownership | Not defined | DeepSeek | Minor | **Accepted** | Documented in ADR-022: drivers own private state, DriverManager owns instances, DeviceGraph owns topology only. |
| 19 | Timeline estimates | Missing effort estimates | DeepSeek | Minor | **Rejected** | Research OS with single developer — estimates are noise. Milestone dependency graph is sufficient. |
| 20 | Storage type | SPI NAND vs eMMC unspecified | DeepSeek | Minor | **Accepted** | P3.5a: SPI NAND primary. P3.5b: eMMC secondary, if available on target. |

## Resulting Artifacts

| Artifact | File | Purpose |
|----------|------|---------|
| ADR-020 | `docs/adr/ADR-020-system-runtime-ownership.md` | SystemState ownership rules |
| ADR-021 | `docs/adr/ADR-021-bootinfo-escape-prevention.md` | BootInfo escape prevention |
| ADR-022 | `docs/adr/ADR-022-driver-lifecycle-contract.md` | Minimal Driver lifecycle contract |
| ADR-023 | `docs/adr/ADR-023-identity-state-model.md` | IdentityState enum model |
| ADR-011 amend. | `docs/adr/ADR-011-phase-transition.md` | Frozen component unfreezing criteria |
| Roadmap v1.1 | `docs/architecture/master-roadmap.md` | Updated with V-epic structure |
| State update | `PROJECT_STATE.md` | Updated milestones + ADR table |

## Key Rejections (with rationale)

| Rejected Proposal | Rationale |
|-------------------|-----------|
| Cargo aliases for old package names | Transient compatibility abstraction (ADR-011). Rename is one-time, scripted, with git checkpoints. |
| Full Identity type hierarchy | Premature: only two concrete states exist. Enum sufficient (ADR-023). |
| Volatile recovery prototype | Recovery without persistence is testing infrastructure, not architecture. V6 remains blocked on V1.2b. |
| Formal V2.0 Memory review milestone | Excessive for porting already-validated code. ADR-011 amendment sufficient. |
| Early RPi3 parallel target | Single developer cannot maintain 2+ physical targets. RPi3 after V3 Device Graph. |
| Remove Driver trait | Three existing entities (UART, GIC, Timer) share lifecycle. Trait is contract, not abstraction. |
| Timeline effort estimates | Research OS + single dev — estimates are noise. Milestone graph sufficient. |
