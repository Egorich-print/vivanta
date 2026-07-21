# Project State

> **Current baseline:** V0.1
> **Last updated:** 2026-07-21

## Quick Reference

| Field | Value |
|-------|-------|
| Version | V0.1 complete |
| Current milestone | V1.1 Runtime Identity (planning) |
| Main dev platform | QEMU AArch64 |
| Hardware bring-up | RK3568 Stage 1, RPi3 B+ Stage 1 |
| Architecture status | V0 architecture stabilized |
| Major blockers | RK3568 BootInfo assembly, RPi3 UART firmware, Storage driver |

---

## 1. Project Overview

Vivanta (formerly TheseusOS) is an operating system that preserves its identity and user environment across complete replacement of its hardware components. The core philosophy is minimizing friction between users and hardware evolution.

The central innovation is not a new kernel or driver model, but a formal protocol for system identity persistence across physical hardware transitions — **cryptographic continuity**: a system proves it is the same entity across hardware changes through a verifiable chain of signed State Documents.

The definitive order of development is governed by the [Master Roadmap](docs/architecture/master-roadmap.md).

## 2. Current Development Stage

| Stage | Status | Notes |
|-------|--------|-------|
| V0 | Complete | Architecture foundation, rename TheseusOS → Vivanta |
| V0.1 | Complete | SystemState + Identity Bootstrap |
| V1.1 | Planning | Runtime Identity — BootInfo migration, identity transfer |
| V1.2b | Blocked | Persistent Identity — requires storage layer |
| V2 / M5 | Planned | Memory Resource Manager |
| V3 | Planned | Device Graph + Driver Contract |

## 3. Repository Layout

```
vivanta/
├── docs/              # Documentation (roadmap, ADRs, audit, hardware)
├── specs/             # RFCs and schemas
├── archive/           # Historical experiments and decisions
└── vivanta-boot/      # Active Cargo workspace
    ├── boot_info/     # BootInfo contract types
    ├── boot_common/   # Console, println!, FDT scanner
    ├── arch-*/        # Architecture implementations (aarch64, armv7a, test-stub)
    ├── kernel/        # Kernel (PMM, VMM, Scheduler, SystemState)
    ├── platform-*/    # Platform support (qemu, rk3568, sdm660, allwinner)
    └── target-*/      # Target binaries (qemu, rk3568, rpi3b-plus, etc.)
```

## 4. Architecture Status

| Aspect | Status |
|--------|--------|
| V0 architecture | Stabilized |
| Execution Foundation | Complete (M4: cooperative threads, timer, GIC) |
| Identity Bootstrap | Complete (V0.1: SystemState, IdentityState) |
| ADR baseline | ADR-011 through ADR-023 |
| Post-audit additions | ADR-020 through ADR-023 (accepted July 2026) |
| RFC baseline | RFC-001 through RFC-008 accepted; RFC-009–010 pending |
| Build-time verification | kernel links with arch-test-stub (no real ISA) |

## 5. Platform Status

| Platform | Status | What works | Next step |
|----------|--------|------------|-----------|
| QEMU AArch64 | Working | Full boot → kernel_main → EL0 → yield | Maintenance |
| RK3568 | Stage 1 | UART writes, SystemState, EL2→EL1 | BootInfo → kernel_main |
| RPi3 B+ | Stage 1 | PL011 UART writes '.' marker | UART validation, DTB parse |
| X96Q | Planned | Platform crate exists | Build + UART init |
| Lavender (SDM660) | Early | Console init + boot banner | Deferred (RK3568 prioritized) |

## 6. Current Milestone: V1.1 Runtime Identity

**Goal:** Establish the runtime identity framework — make identity accessible through SystemState, prepare for persistent identity.

**Done when:**
- [ ] Runtime identity exists and is accessible through SystemState
- [ ] Boot identity transfer from bootloader to kernel works
- [ ] Identity available through SystemState at kernel_main entry
- [ ] Volatile → Persistent transition enum is wired (even if Persistent is stub)
- [ ] RK3568 adapter_main calls kernel_main with BootInfo

**Unblocked:**
- Runtime identity (Volatile)
- SystemState integration
- Boot identity transfer

**Blocked:**
- Persistent identity implementation requires storage layer (P3.5)

**Fallback:** Continue on QEMU if hardware bring-up is blocked.

## 7. Known Blockers

| Blocker | Impact | Path forward |
|---------|--------|--------------|
| RK3568 BootInfo assembly | kernel_main not called on real hardware | Complete adapter_main boot flow |
| RPi3 UART firmware | No serial output on real hardware | Verify firmware compatibility, test with known-good OS |
| Storage driver (P3.5) | Persistent identity impossible | SPI NAND or eMMC driver needed |
| Preemptive context switch | Blocked on QEMU | Validate on RK3568 hardware |
| Thread stack leak | thread_exit() does not reclaim frames | Deferred to M5.x |

## 8. Next Milestones

| Milestone | Priority | Summary |
|-----------|----------|---------|
| V1.1 | P1 | Runtime Identity — identity accessible through SystemState |
| V1.2b | P1 | Persistent Identity — blocked on storage driver |
| V2 / M5 | P2 | Memory Resource Manager — integrate existing MemoryObject |
| V3 | P3 | Device Graph + minimal Driver contract (ADR-022) |

---

## 9. Reference Material

### 9.1 Vision, Philosophy, and Invariants

**Vision:** Vivanta is an operating system that preserves its identity and user environment across complete replacement of its hardware components.

**Philosophy:** The project is guided by the "Ship of Theseus" principle — a system can survive complete replacement of its parts if its identity and history are preserved.

Key tenets:

- **User First**: The system exists to serve the user, not the other way around.
- **Adaptive System**: The OS adapts to hardware; hardware does not dictate OS limitations.
- **Architecture Independence**: The core platform must be portable across major architectures (x86_64, ARM, RISC-V).
- **Minimal Friction**: The system makes technical decisions automatically; advanced users can override defaults.
- **Document Before Code**: Architectural decisions are documented before implementation.
- **Modularity and Composability**: The system is built from independent, composable components.

**Core Invariants:**

1. **User Environment Preservation**: User's applications, data, and settings must persist across hardware changes.
2. **Hardware Adaptability**: The OS must adapt to hardware; hardware should not dictate OS limitations.
3. **Architecture Independence**: The core platform must be portable across major architectures.
4. **Minimal Friction**: System automates decisions; reduces manual user configuration.
5. **Documentation as Source of Truth**: Architecture is documented before implementation.
6. **Long-Term Maintainability**: Architecture must be designed for evolution over decades.
7. **Identity Independence**: The Root Keypair must not depend on the component it is designed to survive replacement of.
8. **No Booting in Unknown State**: If identity cannot be resolved, the system halts rather than booting in an indeterminate state.

### 9.2 Completed RFCs

| RFC | Title | Status | Validated By |
|-----|-------|--------|-------------|
| RFC-001 | Identity Model | Accepted | M1-A experiment |
| RFC-001.5 | Identity Utility Model | Accepted | M1-A experiment |
| RFC-002 | Bootstrap Architecture | Accepted | M1-A experiment |
| RFC-003 | Boot Protocol | Accepted | M1-A experiment |
| RFC-004 | Recovery Seed Format | Accepted | M1-A experiment |
| RFC-005 | State Document Format | Accepted | M1-A experiment |
| RFC-006 | Environment Continuity Model | Accepted | M2-A experiment |
| RFC-007 | Dynamic Device Tree and Hardware Adaptation | Draft | M1-B (pending) |
| RFC-008 | Boot Protocol (revised) | Accepted | AArch64 + ARMv7 boot |
| RFC-009 | Platform Capability Model | Accepted | Architecture design |
| RFC-010 | Memory Resource Model | Experimental | Awaiting hardware validation |

### 9.3 Terminology

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

### 9.4 ADRs (July 2026)

| ADR | Title | Status |
|-----|-------|--------|
| ADR-020 | System Runtime Ownership | Accepted |
| ADR-021 | BootInfo Escape Prevention | Accepted |
| ADR-022 | Minimal Driver Lifecycle Contract | Accepted |
| ADR-023 | IdentityState Model | Accepted |
| ADR-011 | (Amendment) Frozen Component Unfreezing | Amended |

### 9.5 Validated by Experiment

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

### 9.6 Key Architectural Decisions

1. Identity is cryptographic (Ed25519 keypair), not a UUID or hostname.
2. Continuity is formal: same keypair + verified State Chain = same system.
3. Identity must be independent of the component it outlives (Recovery Seed).
4. No booting in unknown identity state (safety halt).
5. Boot protocol has 5 stages with 3 modes: Genesis, Normal, Recovery.
6. M1 proves one thing: storage replacement without reinstallation.
7. M1-A (QEMU) before M1-B (hardware).

### 9.7 Architecture Constraints

- **Primary Language**: Rust (`#![no_std]`). Confirmed during M1-A.
- **Initial Target**: QEMU AArch64 — Complete. RK3568 — Stage 1. Lavender — Deferred.
- **State Document Format**: JSON (CBOR deferred).
- **Recovery Seed Format**: BIP-39 12-word mnemonic.
- **Signature Algorithm**: Ed25519.
- **Hash Function**: SHA-256.
- **Identity Derivation**: Root Keypair is derived from the recovery seed, not generated independently. The seed is the single root of truth.

### 9.8 Open Research Topics

- Fork detection and Divorce Statements (deferred past M1).
- Ownership transfer and multi-device identity (deferred past M1).
- Autonomous agent continuity (deferred past M1).
- Secure element / TPM key storage (deferred past M1).
- Encrypted State Documents (deferred past M1).
- Bootloader-level identity verification (deferred past M1).

---

## Document Purpose

This file describes the **current state** of Vivanta. It is the primary entry point for new contributors and language models.

Historical decisions and detailed specifications are stored in:
- `docs/adr/` — Architecture Decision Records
- `docs/milestones/` — Milestone acceptance criteria and reviews
- `specs/rfc/` — Request for Comments
- `archive/` — Historical experiments, decisions, and research

This document should be updated at milestone boundaries, not after every commit.
