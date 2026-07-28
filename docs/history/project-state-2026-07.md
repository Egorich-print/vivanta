# Project State

> **Current baseline:** V2/M5 (Phase A–E complete), M1.0, M1.1 complete
> **Last updated:** 2026-07-27

## Quick Reference

| Field | Value |
|-------|-------|
| Version | V1.1 complete, V2/M5 active, M1.0/M1.1 complete |
| Current milestone | V2/M5 Memory Resource Manager — Phases A–E done, smoke test pending |
| Main dev platform | QEMU AArch64 |
| Hardware bring-up | RK3568 Stage 1, RPi3 B+ Stage 1 |
| Architecture status | V1 stabilized (ADR-021/024, ADR-025), M1.0/M1.1: early MMU + independent PMM |
| Major blockers | Storage driver |

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
| V1.1 | Complete | Runtime Identity — ADR-021/024 ratified, BootInfo escape closed |
| V2 / M5 | Active | Memory Resource Manager — Phases A–E complete (2026-07-24) |
| V2 / M6 | Planned | Resource-backed Runtime — first isolated EL0 process |
| V1.2b | Blocked | Persistent Identity — requires storage layer (P3.5) |
| V3 | Planned | Device Graph + Driver Contract |

### 2a. OS Maturity Assessment (2026-07-24)

Vivanta is an **early-stage kernel** — past bare-metal bring-up, firmly in foundation construction, but a long way from being a usable operating system.

#### What the OS CAN do

| Capability | Details |
|------------|---------|
| **Boot** | Full boot on QEMU AArch64 and RK3568 hardware: UART init → FDT parse → BootInfo → kernel_main |
| **CPU** | Exception vectors, FP/SIMD init, EL2→EL1 transition, WFI idle |
| **MMU** | 4-level AArch64 page tables, identity-mapped RAM, MMIO device mappings, runtime map/unmap, **early boot identity map** (static 4 GB, 2 MB blocks, no allocator) — new in M1.0 |
| **Memory** | Bitmap physical frame allocator (PmmBitmap), Memory Resource Manager with PmmBackend, MemoryObject lifecycle (create/allocate/map/share/revoke), placement policy scoring engine, **memory discovery** (BootInfo → usable regions) |
| **VMM** | Multi-address-space model (kernel + user ASes), hardware isolation verified, TTBR0_EL1 switching on context switch |
| **Scheduler** | Cooperative round-robin, boot thread + user threads, yield-based switching |
| **Interrupts** | GICv3 init + enable, ARM generic timer with tick counting, timer IRQ handler registered |
| **User mode** | EL0 entry via `eret_to_user_stub`, user code page mapped, user stack, SVC syscall roundtrip (EL0→EL1→EL0) |
| **Identity** | SystemState with RuntimeIdentity, BootIdentity→Runtime transition, UUID + generation counter, volatile/permanent enum |
| **Architecture** | Kernel/arch/platform split (ACS), `extern "Rust"` boundary, arch-test-stub for build-time ISA independence |
| **UART** | Shared `Console` trait, `PL011` and `NS16550` drivers in `boot_common`, `platform-rpi3b` crate with GPIO init, shared `println!` macro |
| **Diagnostics** | UART console with `println!`, memory map print, PMM stats, MRM backend list |

#### What the OS CANNOT do (yet)

| Gap | Milestone | What's needed |
|-----|-----------|---------------|
| **Persistent identity** | V1.2b (blocked) | Storage driver (SPI NAND/eMMC) to store identity across reboots |
| **Device drivers** | V3 | Device Graph + `trait Driver` contract — no MMIO device driver framework exists |
| **User processes** | V4 | Kernel Task object (Thread + Address Space + Resources), process model, IPC |
| **Filesystem** | Post-V7 | No VFS, no block cache, no file abstraction |
| **Networking** | Post-V7 | No TCP/IP stack, no NIC drivers |
| **SMP** | Out of scope | Single-core only (cooperative scheduler, no per-CPU data structures) |
| **Preemptive scheduling** | Deferred | Currently cooperative yield only; timer IRQ exists but scheduler doesn't preempt |
| **Dynamic memory allocation** | V2.x | `StubAllocator` panics — no heap, no Vec/Box at runtime |
| **User I/O** | — | Not even a `sys_write` to UART for user threads |
| **ELF loading** | — | No ELF parser, no user binary loader |
| **Shell** | — | Don't even ask |

#### Where Vivanta fits on the OS maturity spectrum

```
Bootloader stage  →  Early kernel  →  Driver framework  →  Userspace  →  Self-hosting
                         ↑
                    Vivanta is here
```

Comparable to: Linux after `start_kernel()` but before `init` — the kernel is alive (MMU on, scheduler running, memory managed) but there are no user processes, no drivers, no filesystem, no I/O beyond the serial console.

#### What makes Vivanta different from other hobby kernels at this stage

1. **Identity-first architecture** — the cryptographic identity model (Ed25519 keypair, State Documents, Recovery Seed) is designed BEFORE filesystems/drivers, not bolted on later
2. **Arch-kernel split (ACS)** — kernel compiles against `arch-test-stub`, proving zero ISA dependencies at build time. Portable to x86_64/RISC-V without kernel changes
3. **Memory as a resource** — MemoryObject/MemoryBackend/PlacementPolicy model is designed for heterogeneous memory (DDR, HBM, CXL, persistent) from day one, not as an afterthought
4. **Capability-based access** — MemoryCapability/MemRights model is wired into MemoryObject, ready for capability derivation when IPC exists
5. **Document-before-code** — ADRs ratified before implementation (ADR-001 through ADR-025)

## 3. Repository Layout

```
vivanta/
├── docs/              # Documentation (roadmap, ADRs, audit, hardware)
├── specs/             # RFCs and schemas
├── archive/           # Historical experiments and decisions
└── vivanta-boot/      # Active Cargo workspace
    ├── boot_info/     # BootInfo contract types
    ├── boot_common/   # Console, println!, FDT scanner, PL011, NS16550, memory_discovery
    ├── arch-*/        # Architecture implementations (aarch64, armv7a, test-stub)
    │   └── aarch64/   # early_mmu.rs — static identity map
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
| Runtime Identity | Complete (V1.1: BootInfo escape, SystemState encapsulation) |
| Early MMU (M1.0) | Complete (early_mmu.rs — static identity map 4 GB, 2 MB blocks, no allocator) |
| Physical Memory Manager (M1.1) | Complete (PmmBitmap — bitmap allocator, self-test, counters, decoupled from BootInfo) |
| ADR baseline | ADR-011 through ADR-025 |
| Post-audit additions | ADR-020 through ADR-025 (accepted July 2026) |
| RFC baseline | RFC-001 through RFC-008 accepted; RFC-009–010 pending |
| Build-time verification | kernel links with arch-test-stub (no real ISA) |

## 5. Platform Status

| Platform | Status | What works | Next step |
|----------|--------|------------|-----------|
| QEMU AArch64 | Working | Full boot → kernel_main → EL0 → yield | Maintenance |
| RK3568 | Stage 1 | UART writes, SystemState, EL2→EL1 | BootInfo → kernel_main |
| RPi3 B+ | Stage 1 | PL011 UART via shared driver, GPIO init, `println!` via `Console`, identity MMU map (4 GB) | Hardware validation, DTB parse → BootInfo → kernel_main |
| X96Q | Planned | Platform crate exists | Build + UART init |
| Lavender (SDM660) | Early | Console init + boot banner | Deferred (RK3568 prioritized) |

## 6. Completed Milestone: V1.1 Runtime Identity

**Goal:** Establish the runtime identity framework — make identity accessible through SystemState, prepare for persistent identity.

**Completed:** 2026-07-23

**What was done:**
- ADR-021: System State Encapsulation — all SystemState fields private, accessed via getters
- ADR-024: Identity Model Separation — BootIdentity, RuntimeIdentity, PersistentIdentity types
- BootInfo escape eliminated — `kernel_main` no longer accesses `info.*` after `from_boot_info()`
- Common boot flow for all platforms: `_start → adapter_main → FDT → BootInfo → kernel_main`
- RK3568 brought onto standard boot flow (was using direct SystemState construction)
- Volatile → Persistent transition enum wired (IdentityState::Runtime / Persistent)
- `Eq` derives added to all identity types

**Exit criteria met:**
- [x] Runtime identity exists and is accessible through SystemState
- [x] Boot identity transfer from bootloader to kernel works
- [x] Identity available through SystemState at kernel_main entry
- [x] Volatile → Persistent transition enum is wired (even if Persistent is stub)
- [x] RK3568 adapter_main calls kernel_main with BootInfo

## 7. Current Milestone: V2/M5 Memory Resource Manager (active)

**Goal:** Integrate existing MemoryObject prototype into the kernel as a formal Memory Resource Manager per master-roadmap P2.

**ADR:** ADR-025 (Memory Resource Manager Integration) — proposed 2026-07-24.

**Status (2026-07-24):** Implementation phases A–E complete. Compiles on all active targets.

| Phase | Description | Status |
|-------|-------------|--------|
| A | PmmBackend adapter — wraps PmmBitmap as MemoryBackend | ✅ Done |
| B | MRM in SystemState — merged BootMemoryManager, removed from pmm.rs | ✅ Done |
| C | Runtime page table writer — `mmu_map_object` / `mmu_unmap` in arch-api + aarch64 | ✅ Done |
| D | MemoryObject::map() → programs real MMU via arch-api | ✅ Done |
| E | Full workspace build — QEMU, RK3568, test-stub compile | ✅ Done |
| F | QEMU smoke test — allocate MemoryObject, map, write, read, unmap | ⏳ Pending |

**Files created:**
- `kernel/src/memory/{mod,resource,capability,object,policy,manager,pmm_backend}.rs`
- `docs/adr/ADR-025-memory-resource-manager-integration.md`

**Files modified:**
- `kernel/src/state/mod.rs` — added `memory_manager` field + `init_memory()`
- `kernel/src/pmm.rs` — removed `BootMemoryManager`, added `region_start()` getter
- `kernel/src/lib.rs` — replaced BootMemoryManager with direct PMM + MRM init
- `arch-api/src/mmu.rs` — added `mmu_map_object`, `mmu_unmap` declarations
- `arch-aarch64/src/mmu.rs` — implemented runtime page table walk + TLBI
- `arch-test-stub/src/lib.rs` — added stubs for new runtime MMU functions

**Known limitations (V2/M5 scope):**
- `mmu_map_object` panics on L2 block descriptors — requires 2MB block splitting for general use
- No smoke test yet (needs VADDR with existing L3 tables, e.g. 0x0900_1000 on QEMU)
- `MemoryBackend` trait has no `reserve()` method — boot-time reservation done via `PmmBitmap::reserve()` directly
- Stub global allocator still panics — real allocator deferred to V2.x follow-up

**Depends on:**
- Nothing; QEMU is sufficient for all remaining V2/M5 work

**Blocked by:**
- Nothing

## 8. Known Blockers

| Blocker | Impact | Path forward |
|---------|--------|--------------|
| RPi3 UART firmware | No serial output on real hardware | Verify firmware compatibility, test with known-good OS |
| Storage driver (P3.5) | Persistent identity impossible | SPI NAND or eMMC driver needed |
| Preemptive context switch | Blocked on QEMU | Validate on RK3568 hardware |
| Thread stack leak | thread_exit() does not reclaim frames | Deferred to M5.x |

## 9. Next Milestones

See [OS Maturity Assessment](docs/OS_MATURITY.md) for a detailed architectural analysis.

| Milestone | Priority | Summary |
|-----------|----------|---------|
| V2 / M5 completion | P0 | Smoke test MemoryObject + L2 block splitting |
| V2 / M6 | P1 | Resource-backed Runtime — first isolated EL0 process |
| V3 | P2 | Device Graph — describe hardware world, capability-wired |
| V4 | P2 | Kernel Task — spawn/exit/yield, first real process model |
| V5 | P1 | Storage Identity — PersistentIdentity closes ADR-024 loop |
| V1.2b | P1 | Persistent Identity — blocked on storage driver (P3.5) |

---

## 10. Reference Material

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
| ADR-021 | System State Encapsulation | Accepted |
| ADR-022 | Minimal Driver Lifecycle Contract | Accepted |
| ADR-023 | IdentityState Model | Accepted |
| ADR-024 | Identity Model Separation | Accepted |
| ADR-025 | Memory Resource Manager Integration | Proposed |
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
