# Vivanta OS Maturity Assessment

> **Date:** 2026-07-24
> **Current baseline:** V2/M5 Phase A–E complete
> **Reference:** [STATUS.md](../STATUS.md), [master-roadmap.md](architecture/master-roadmap.md)

---

## 1. Current State

Vivanta V2/M5 completes the transition from "a kernel that works" to **"a kernel that manages resources"**.

### Kernel lifecycle (fully operational)

```
Boot ROM → Bootloader → UART/FDT → BootInfo → kernel_main()
                                                    ↓
                                              SystemState
                                                    ↓
                                              PMM init → MRM init
                                                    ↓
                                              MMU active
                                                    ↓
                                              GIC/timer active
                                                    ↓
                                              Scheduler active
                                                    ↓
                                              EL0 available
```

This is not a bare-metal monitor. This is a **kernel runtime**.

### The key architectural transition of M5

Before M5, memory was infrastructure:

```
Kernel
 ├── PMM
 ├── VMM
 ├── Scheduler
 └── BootInfo
```

After M5, memory is a **system resource**:

```
SystemState
 ├── Identity
 ├── MemoryResourceManager
 │    ├── MemoryObject (lifecycle FSM)
 │    ├── MemoryCapability (access rights)
 │    ├── MemoryBackend (heterogeneous memory)
 │    └── PlacementPolicy (scoring engine)
 ├── Scheduler
 ├── [future] DeviceGraph
 └── [future] Storage
```

### M5 — Architectural Point of No Return

Before M5, Vivanta could have evolved into a classic hobby kernel:

```
boot → memory → drivers → processes → shell
```

After M5, the direction changed irreversibly:

```
SystemState
     │
     ├── Identity
     │
     └── Resource Managers
           ├── Memory
           ├── Device
           ├── Storage
           └── Execution
```

This is no longer a set of subsystems. It's a **resource ownership model**.

The fundamental change:

| Before | After |
|--------|-------|
| "The kernel has memory" | "The kernel owns resources; memory is one of them" |

This distinction propagates into every future component. A `Task` won't be a container for code — it will be an owner of resources (`AddressSpace`, `MemoryCapabilities`, `DeviceCapabilities`, `StorageCapabilities`). A `Device` won't be a function that writes to MMIO — it will be a node with resources and capabilities.

## 2. Maturity: ~35-40%

Not by lines of code — by fundamental subsystems required for a minimal self-sufficient OS.

### Completed

| # | Subsystem | Status |
|---|-----------|--------|
| 1 | Boot chain (ROM → BootInfo → kernel_main) | ✅ |
| 2 | MMU (4-level page tables, runtime map/unmap) | ✅ |
| 3 | Physical Memory Manager (bitmap allocator) | ✅ |
| 4 | Memory Resource Manager (MemoryObject, capability, policy) | ✅ |
| 5 | Virtual Memory (multi-address-space, isolation) | ✅ |
| 6 | Scheduler (cooperative round-robin, user threads) | ✅ |
| 7 | Interrupts (GICv3, timer IRQ) | ✅ |
| 8 | EL0 entry / SVC roundtrip | ✅ |
| 9 | Runtime Identity (SystemState, BootIdentity→Runtime) | ✅ |
| 10 | Arch-kernel split (ACS, extern "Rust" boundary) | ✅ |

### Remaining

| # | Subsystem | Estimated milestone |
|---|-----------|---------------------|
| 11 | MemoryObject smoke test + L2 block splitting | V2/M5 completion |
| 12 | Isolated EL0 runtime environment | V2/M6 |
| 13 | Device Graph | V3 |
| 14 | Kernel Task model (first real process) | V4 |
| 15 | Storage + Persistent Identity | V5 |
| 16 | Service Framework | V5+ |
| 17 | Recovery Manager | V6 |
| 18 | Filesystem, Networking, Userspace | V7+ |

The next psychological threshold: **Vivanta creates its first isolated EL0 runtime from its own resources through MRM**. After that, it's not just a kernel — it's an OS in embryonic form.

## 3. Architectural Lineage

Measured by architectural philosophy, not code volume:

```
Minix 3        ───┐
                   │
seL4           ───┼── Vivanta direction
                   │
QNX            ───┘

Linux          ─── separate lineage
```

Why:
- **MemoryObject** ≈ microkernel object model (resources, not functions)
- **Capability layer** (MemRights, MemoryCapability) ≈ seL4/QNX capability derivation
- **Device Graph** → explicit resource topology, not a flat device list
- **SystemState** → controlled kernel context, not global mutable state

## 4. What NOT to do now

### SMP

Tempting but wrong. SMP requires: per-CPU data, locking, IPI, TLB shootdown, scheduler migration. Vivanta currently has no persistent identity, no storage, no device discovery. SMP provides zero user value at this stage.

### Full heap allocator

`StubAllocator` is ugly but the problem is deeper:

```
Box/Vec/String
      ↓
Global allocator
      ↓
MemoryObject        ← THIS is the right boundary
      ↓
MRM
```

If a bump allocator is bolted directly onto PMM, it bypasses MRM. Later, the heap would need to be retrofitted as an MRM consumer. Better to define first:

```
KernelHeap
 ├── MemoryObject (backed by MRM)
 ├── AllocationCapability
 └── Lifetime policy
```

The heap should be an MRM consumer, not an MRM bypass.

### Shell / filesystem / drivers

These come AFTER the system can create isolated EL0 runtimes. Building UX before the runtime foundation exists produces a fragile stack.

## 5. Correct Development Order

```
MemoryObject ──→ DeviceGraph ──→ Task model ──→ Storage identity ──→ Userspace
```

NOT:

```
Storage ──→ Drivers ──→ Shell ──→ Apps ──→ chaotic OS
```

The first path builds architectural integrity. The second builds features on sand.

## 6. Architectural Principle

After M5, Vivanta operates under a unified principle:

> **Every resource must have an owner, a lifetime, and an authority boundary.**

This gives every future subsystem a predictable shape:

| Domain    | Object         | Owner   | Rights         | Lifetime         |
|-----------|----------------|---------|----------------|------------------|
| Memory    | MemoryObject   | Task/MRM| MemRights      | allocate→revoke  |
| Device    | DeviceObject   | System  | Capability     | discover→release |
| Task      | TaskObject     | System  | Resources      | spawn→exit       |
| Storage   | StorageObject  | Task    | Identity       | persist→recover  |
| Identity  | IdentityState  | System  | Root Keypair   | boot→generations |

This principle acts as a design constraint: if a new feature can't answer "who owns this, how long does it live, who can access it?", it doesn't belong in the kernel.

## 7. Next Milestones

### V2/M5 — Complete M5

| Task | Why critical |
|------|-------------|
| MemoryObject smoke test (allocate → map → write → read → unmap) | Validates entire MRM→MMU→TLB→CPU chain exists |
| L2 block descriptor splitting | Enables page-granular permissions, COW, user memory, mmap. Current panic on L2 blocks is a structural debt |

### V2/M6 — Resource-backed Runtime

**Goal:** The kernel creates the first autonomous runtime environment.

This is not "add processes." Processes that lack a resource ownership model are containers of code — fragile, flat, bypassable.

The milestone is stricter:

> **The kernel creates a runtime where all resources are obtained through the system ownership model. No hidden bypasses.**

The creation path:

```
kernel_main()
       │
       ▼
create_task()
       │
       ├── allocate MemoryObject (via MRM)
       ├── create AddressSpace (via VMM)
       ├── map pages (via arch-api mmu_map_object)
       ├── assign Identity (from SystemState)
       └── enter EL0
                 │
                 ▼
           user code runs
```

**Exit criteria — no bypasses:**

| ❌ Forbidden | ✅ Required |
|-------------|------------|
| Global static memory | MemoryObject from MRM |
| Direct PMM access by user | Capability-checked access |
| Hardcoded physical addresses | VMM-allocated virtual addresses |
| UART/MMIO direct access | DeviceCapability-mediated I/O |
| Raw pointers across EL boundary | ArchContext-mediated entry/exit |

Minimum API: `spawn()`, `exit()`, `yield()` — without ELF. A static binary blob in EL0, with its stack and code pages backed by MemoryObject, is the first real process.

### V3 — Device Graph

Not drivers immediately. First: describe the hardware world.

```
DeviceGraph
 ├── UART node (MMIO resource + capability)
 ├── GIC node
 ├── Timer node
 ├── MMIO region nodes
 └── [future] Storage controller node
```

Currently Vivanta knows "I have a UART." It needs to know "I have Device Node UART with resources and capability."

### V4 — Kernel Task

First real process model:

```
kernel → task_manager → EL0 task { stack, address_space, syscall }
```

### V5 — Storage + Persistent Identity

Here the ADR-024 loop closes:

```
BootIdentity → RuntimeIdentity → PersistentIdentity → Storage → Filesystem
```

Vivanta becomes an OS with continuity — not just a kernel that reboots into amnesia.

## 8. Summary

Vivanta has passed the most dangerous phase. The riskiest components — boot, MMU, PMM, interrupt model, execution context, arch separation — all exist and work.

**35-40% done.** The hardest 40% was getting the foundation right. The remaining 60% is quantitatively harder but qualitatively predictable: the architectural decisions are already made, the boundaries are drawn, and each new subsystem plugs into a known interface (MRM, arch-api, SystemState).

The next checkpoint after M6 is not "another subsystem done." It is the moment when:

> Vivanta can not only boot and manage hardware, but **create its own entity of execution** inside its own resource model.

That will be the first real transition from **kernel foundation** to **operating system**.
