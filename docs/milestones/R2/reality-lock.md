# R2 Reality Lock

**Goal:** Transition from simulated architecture to physical hardware.

**Target:** Redmi Note 7 (lavender, SDM660)
**Architecture:** ARM64 (Cortex-A73 + Cortex-A53)
**Boot:** Android boot.img chain
**Debug:** UART

---

## Phases

### R2.0 Repository Foundation ✅ Complete

- docs/architecture/repository-layout.md created
- Milestones, RFCs, ADRs moved to correct locations
- Historical artifacts archived
- .gitignore added

### R2.1 Architecture Stabilization

- PROJECT_STATE.md updated
- RFC freeze documented
- R2 acceptance criteria defined
- Schema hardening (StateDocument + EnvironmentManifest versioning)

### R2.2 Hardware Bringup (M1-B)

#### M1-B0: First Light

Minimal UART output on device. No DTB, no MMU, no allocator.

```
"Theseus alive\n"
```

**Acceptance:** UART output visible via serial adapter.

#### M1-B1: Hardware Discovery

- DTB parsing
- Hardware Inventory construction
- UART console

**Acceptance:** Memory map, CPU count, platform name printed.

#### M1-B2: Minimal Kernel

- Exception vectors
- MMU enable
- PMM (bitmap allocator)
- Heap

**Acceptance:** "MMU enabled successfully" on device UART.

#### M1-B3: MemoryObject on Hardware

- MemoryBackend wrapping PMM
- MemoryObject lifecycle (create, allocate, map, share, revoke)

**Acceptance:** Full MemoryObject lifecycle printed on device UART.

#### M1-B4: Continuity Proof

- Genesis on device
- State chain creation
- Storage death simulation
- Recovery via seed

**Acceptance:** Identity restoration verified on device.

---

## Constraints

- Single developer, one physical device
- No JTAG — UART-only debug
- Boot via `fastboot boot boot.img`
- No kernel/userspace split until after R2

## Non-goals

- Userspace
- Scheduler
- IPC
- IOMMU
- CXL / VRAM / persistent memory
- Encryption
- Distributed identity