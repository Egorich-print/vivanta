# Project State

This document summarizes the durable architectural knowledge of the Vivanta project (formerly Vivanta). It is a living architectural artifact, intended to guide the project's long-term evolution.

The definitive order of development, milestones, and engineering tracks is governed by the [Master Roadmap](docs/architecture/master-roadmap.md), which serves as the core engineering constitution for the project. All architectural decisions and technical sprints must align with the roadmap.

## Vision

Vivanta (formerly Vivanta) is an operating system that preserves its identity and user environment across complete replacement of its hardware components. The core philosophy is minimizing friction between users and hardware evolution.

The project has evolved from "a universal operating system" to a more precise identity: **a continuity-preserving computing platform**. The central innovation is not a new kernel or driver model, but a formal protocol for system identity persistence across physical hardware transitions.

## Philosophy

The project is guided by the "Ship of Theseus" principle: a system can survive complete replacement of its parts if its identity and history are preserved. Key tenets:

*   **User First**: The system exists to serve the user, not the other way around.
*   **Adaptive System**: The OS adapts to hardware; hardware does not dictate OS limitations.
*   **Architecture Independence**: The core platform must be portable across major architectures (x86_64, ARM, RISC-V).
*   **Minimal Friction**: The system makes technical decisions automatically; advanced users can override defaults.
*   **Document Before Code**: Architectural decisions are documented before implementation and serve as the definitive reference.
*   **Modularity and Composability**: The system is built from independent, composable components.

## Core Invariants

These are fundamental, non-negotiable properties that must hold true throughout the project's evolution:

1.  **User Environment Preservation**: User's applications, data, and settings must persist across hardware changes.
2.  **Hardware Adaptability**: The OS must adapt to hardware; hardware should not dictate OS limitations.
3.  **Architecture Independence**: The core platform must be portable across major architectures.
4.  **Minimal Friction**: System automates decisions; reduces manual user configuration.
5.  **Documentation as Source of Truth**: Architecture is documented before implementation.
6.  **Long-Term Maintainability**: Architecture must be designed for evolution over decades.
7.  **Identity Independence**: The Root Keypair must not depend on the component it is designed to survive replacement of.
8.  **No Booting in Unknown State**: If identity cannot be resolved, the system halts rather than booting in an indeterminate state.

## Architecture Identity

The project's resolved identity is:

> **Vivanta is an operating system that preserves its identity and user environment across complete replacement of its hardware components.**

This replaces the earlier "Adaptive Computing Platform" / "Adaptive Operating Platform" framing, which was too abstract. The core concept is **cryptographic continuity**: a system proves it is the same entity across hardware changes through a verifiable chain of signed State Documents.

## Completed RFCs

| RFC | Title | Status | Validated By |
|-----|-------|--------|-------------|
| RFC-001 | Identity Model | ✅ Accepted | M1-A experiment |
| RFC-001.5 | Identity Utility Model | ✅ Accepted | M1-A experiment |
| RFC-002 | Bootstrap Architecture | ✅ Accepted | M1-A experiment |
| RFC-003 | Boot Protocol | ✅ Accepted | M1-A experiment |
| RFC-004 | Recovery Seed Format | ✅ Accepted | M1-A experiment |
| RFC-005 | State Document Format | ✅ Accepted | M1-A experiment |
| RFC-006 | Environment Continuity Model | ✅ Accepted | M2-A experiment |
| RFC-007 | Dynamic Device Tree and Hardware Adaptation | 📝 Draft | M1-B (pending) |
| RFC-008 | Boot Protocol (revised) | ✅ Accepted | AArch64 + ARMv7 boot |
| RFC-009 | Platform Capability Model | ✅ Accepted | Architecture design |
| RFC-010 | Memory Resource Model | 🔬 Experimental | Awaiting hardware validation |

## Terminology

| Term | Definition |
|------|-----------|
| **System Identity** | Ed25519 keypair. The canonical identity of a Vivanta system. |
| **Root Keypair** | The genesis keypair from which all identity claims derive. |
| **Recovery Seed** | BIP-39 mnemonic (12 words) that can regenerate the Root Keypair. |
| **State Document** | Signed CBOR document recording the system's hardware/software inventory at a point in time. |
| **State Chain** | Ordered sequence of State Documents linked by cryptographic hashes. |
| **Genesis State** | State Document 0, created at first boot. The root of the chain. |
| **Continuity** | Property of being the same system (same Root Keypair + verified State Chain). |
| **Fork** | Two systems sharing a Root Keypair but with diverging State Chains. |
| **Divorce Statement** | Signed document establishing a fork as an independent identity. |
| **Boot Protocol** | 5-stage sequence: Bootloader → Identity Check → Identity Resolution → Boot Decision → System Boot. |
| **Identity Independence** | Architectural constraint: the Root Keypair must survive replacement of the component it is stored on. |
| **Environment Manifest** | Signed JSON document recording user data hash, config hash, and software inventory. Peer to State Document. |
| **Environment Chain** | Ordered sequence of Environment Manifests linked by cryptographic hashes, tracking environment continuity independently of the State Chain. |
| **Incremental Update** | Environment Manifest update that occurs between State Document transitions, tracking file changes without hardware migration. |

## Current Milestones

| Phase | Status | Description |
|-------|--------|-------------|
| R0 | ✅ Complete | Peer review, architecture audit, identity resolved |
| RFC Chain | ✅ Complete | RFC-001 through RFC-006 defining the identity and environment protocol |
| M1-A | ✅ Complete | Continuity Proof Experiment on QEMU — core thesis validated |
| M2-A | ✅ Complete | Environment Continuity Experiment — user data persistence validated |
| M3-A | ✅ Complete | Incremental Environment Continuity — tracked changes without state migration |
| M3-B | ✅ Complete | Memory Object Foundation — resource-oriented memory model (QEMU) |
| M3-C | ✅ Complete | Memory Object Semantics — lifecycle, clone, share, revoke |
| M1-B | ⏸️ Deferred | Hardware bringup — Redmi Note 7 (lavender, SDM660) |
| R2 | ✅ Complete | Reality Lock — architecture freeze, repository reorg |
| ACS | ✅ Complete | Architecture Cleanup Sprint — kernel/arch/platform/target split, extern "Rust" contract |
| **M4** | ✅ **Complete** | **Execution Foundation — cooperative multi-threading, timer, thread lifecycle, repository restructuring** |
| M4.4 | ✅ Complete | Address Spaces — multi-AS model with verified hardware isolation |
| M4.4.5 | ✅ Complete | Execution Contract Freeze — unified context switch (ADR-017) |
| M4.5.0 | ✅ Complete | EL0 Transition Preparation — InterruptGuard, eret_to_user_stub |
| M4.5.1 | ✅ Complete | First EL0 entry + SVC roundtrip (QEMU) |
| M4.5.2 | 🔧 In Progress | RK3568 bring-up — println! + DTB on real hardware |

### M4 — Execution Foundation

M4 delivered the first working kernel-thread environment on Vivanta:

- **Cooperative round-robin scheduling** with 3 threads (boot + persistent + terminating)
- **Thread lifecycle management**: `create_kernel_thread`, `thread_exit`, `thread_trampoline`, `cleanup()`, idle thread (WFI)
- **Timer at ~79 Hz** on QEMU (CNTP, IRQ 30, tick counting)
- **Architecture-independent kernel**: verified by `cargo build -p target-test` (kernel + arch-test-stub, no ISA dependency)
- **Repository restructuring**: `boot/` → `archive/boot_legacy/`, `kernel/src/memory/` → `kernel-memory-frozen/` crate
- **5 ADRs documented**: ADR-011 through ADR-015

**Known limitation**: True preemptive context switching is blocked on QEMU (writing to on-stack ExceptionFrame from IRQ prevents subsequent timer IRQs). Validation deferred to physical ARM64 hardware (RK3568). Cooperative switching works correctly.

Details in `docs/milestones/M4/acceptance.md`.

## V-Epic Milestones

V-epics replace the earlier R-phase model as the primary planning structure. See [Master Roadmap](docs/architecture/master-roadmap.md) for full dependency graph.

| V-Epic | Priority | Status | Summary |
|--------|----------|--------|---------|
| V0 | P0 | ✅ Partial | Rename Vivanta → Vivanta (pending), roadmap refresh (✅), docs scaffold (✅) |
| V0.1 | P0 | ✅ Complete | Runtime Identity Bootstrap — SystemState skeleton, IdentityState::Volatile, BootInfo owned copy |
| V1 | P1 | 🔧 Planning | Continuity model — BootInfo migration, boot-time state separation |
| V2 / M5 | P2 | 🔧 Planning | Memory Resource Manager — integrate existing MemoryObject |
| V3 | P3 | 🔧 Planned | Device Graph + minimal Driver contract (ADR-022) |
| V4 | P4 | 🔧 Planned | Task abstraction + Scheduler policies |
| V5 | P5 | 🔧 Planned | Service Framework — LoggingService first |
| V6 | P6 | 🔧 Planned | Recovery Manager |
| V7 | P7 | 🔧 Planned | Additional hardware targets |
| V8 | — | 🔧 Planned | Documentation scaffold |

## V0.1 — Runtime Identity Bootstrap

**Status:** ✅ Complete (2026-07-19)

### Deliverables

- `kernel/src/state/` module: `SystemState { identity, hardware }`
- `IdentityState::Volatile(RuntimeIdentity { boot_id, public_key })` per ADR-023
- `HardwareState` with owned copies from `BootInfo` per ADR-021 (no `&'static` references)
- `SystemState::from_boot_info(&BootInfo) -> Self` — construction at `kernel_main` entry
- No global singleton — `SystemState` is local to `kernel_main`

### Validated

- ✅ SystemState skeleton
- ✅ Volatile IdentityState (per-boot identity generation)
- ✅ BootInfo ownership boundary (data copied, references not retained)
- ✅ Owned HardwareState representation (memory_map + mmio_regions as arrays)
- ✅ ADR-020/021/023 compliance
- ✅ `InterruptControllerInfo` excluded from HardwareState (has `&'static str` — ADR-021)
- ✅ Build: `target-rk3568` + `target-qemu-aarch64` — 0 warnings

### Known limitation (technical debt)

Old kernel init path still reads `BootInfo` directly for transient boot operations
(PMM setup, MMU mapping, GIC init). SystemState is constructed in parallel but
existing subsystems have not been migrated yet. Migration will happen incrementally
in V0.2.

### Pending

- RK3568 runtime parity validation (binary ready, requires power cycle)
- Persistent identity (V1.x, blocked on storage driver)

### ADR compliance

| ADR | Requirement | Status |
|-----|-------------|--------|
| ADR-020 | SystemState owns runtime-coordinated state only | ✅ No global singleton, no drivers/services |
| ADR-020 | HardwareState immutable after construction | ✅ Copy-once at boot |
| ADR-021 | No `&'static` references from BootInfo escape | ✅ All data copied, InterruptControllerInfo excluded |
| ADR-023 | IdentityState as enum, not type hierarchy | ✅ Volatile(Persistent() variants |
| ADR-023 | Monotonic transition | ✅ Only Volatile exists today |

## New ADRs (July 2026)

| ADR | Title | Status |
|-----|-------|--------|
| ADR-020 | System Runtime Ownership | Accepted |
| ADR-021 | BootInfo Escape Prevention | Accepted |
| ADR-022 | Minimal Driver Lifecycle Contract | Accepted |
| ADR-023 | IdentityState Model | Accepted |
| ADR-011 | (Amendment) Frozen Component Unfreezing | Amended |

## M1 Boundary

M1 is defined as a **Continuity Proof Experiment**, not an OS implementation. It validates the Vivanta Continuity Layer:
- Identity generation and recovery
- State Chain management
- Boot mode engine (Genesis → Normal → Recovery)
- Storage replacement continuity

The Vivanta Continuity Layer is architecturally separate from the Operating System Layer (kernel, drivers, filesystem, applications). M1 proves the former. The latter begins after M1 is accepted.

Full acceptance criteria and non-goals are documented in `docs/milestones/M1/A-continuity/acceptance.md`.

Hardware bringup acceptance is documented in `docs/milestones/M1/B-hardware/acceptance.md`.

## Architecture Constraints

*   **Primary Systems Programming Language**: Rust. Confirmed during M1-A. Used for the entire Continuity Proof Experiment.
*   **Initial Implementation Target**: QEMU (M1-A) — ✅ Complete. RK3568 — 🔧 In Progress (M4.5.2). Xiaomi Redmi Note 7 / lavender (M1-B) — deferred; superseded by RK3568.
*   **State Document Format for M1**: JSON (CBOR deferred).
*   **Recovery Seed Format**: BIP-39 12-word mnemonic.
*   **Signature Algorithm**: Ed25519.
*   **Hash Function**: SHA-256.
*   **Identity Derivation**: The Root Keypair is DERIVED FROM the recovery seed, not generated independently. The seed is the single root of truth. This was the critical correction discovered during M1-A.

## Validated by Experiment

The following architectural claims are now experimentally validated between QEMU runs:

| # | Claim | Validated In |
|---|-------|-------------|
| 1 | Ed25519 keypair generation, signing, and verification | M1-A |
| 2 | BIP-39 seed → deterministic keypair derivation (same seed → same keypair) | M1-A |
| 3 | State Document creation, signing, signature verification | M1-A |
| 4 | State Chain linkage and chain verification | M1-A |
| 5 | Full recovery flow: seed entry → keypair restoration → public key match → continuity proof | M1-A |
| 6 | Three boot modes (Genesis, Normal, Recovery) as a valid state machine | M1-A |
| 7 | Identity independence from storage (keypair lives in seed, not on storage) | M1-A |
| 8 | Environment Manifest creation, signing, and verification | M2-A |
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
| 22 | extern "Rust" bidirectional contract between kernel and arch | ACS |
| 23 | MMIO addresses moved from kernel to platform (BootInfo.mmio_regions) | ACS |
| 24 | Build-time proof: kernel links with arch-test-stub (no real ISA) | ACS |
| 25 | boot-info crate: zero dependencies, core-only contract types | ACS |

## Key Architectural Decisions

1. Identity is cryptographic (Ed25519 keypair), not a UUID or hostname.
2. Continuity is formal: same keypair + verified State Chain = same system.
3. Identity must be independent of the component it outlives (Recovery Seed).
4. No booting in unknown identity state (safety halt).
5. Boot protocol has 5 stages with 3 modes: Genesis, Normal, Recovery.
6. M1 proves one thing: storage replacement without reinstallation.
7. M1-A (QEMU) before M1-B (hardware).

## Open Research Topics

*   Fork detection and Divorce Statements (deferred past M1).
*   Ownership transfer and multi-device identity (deferred past M1).
*   Autonomous agent continuity (deferred past M1).
*   Secure element / TPM key storage (deferred past M1).
*   Encrypted State Documents (deferred past M1).
*   Bootloader-level identity verification (deferred past M1).

---

This document is a living architectural artifact. Future work should rely on this file as the primary source of context.
