# ADR-025: Memory Resource Manager Integration

## Status
Proposed

## Date
2026-07-24

## Related ADRs
- ADR-011 (Amendment: Frozen Component Unfreezing)
- ADR-014 (Architectural Boundaries)
- ADR-015 (Arch Boundary Contracts — `extern "Rust"`)
- ADR-020 (System Runtime Ownership)
- ADR-021 (System State Encapsulation)
- ADR-024 (Identity Model Separation)

## Context

Vivanta has a validated MemoryObject prototype in `kernel-memory-frozen/` (RFC-010, validated by M3-BC experiment) and a working PMM (`PmmBitmap` in `kernel/src/pmm.rs`). The current `BootMemoryManager` manages boot-time reservation then hands off to `PmmBitmap` for runtime allocation. There is no resource-level memory abstraction — allocations are raw frame requests, no lifecycle, no placement policy, no capability enforcement.

V2/M5 (per master-roadmap P2) requires integrating the MemoryObject prototype into the kernel. The prototype was developed before ADR-021/024, before the arch-kernel split (ACS), and before `SystemState` existed. It must be adapted to the current architecture, not copied verbatim.

### Current Architecture

```
BootMemoryManager → PmmBitmap (FrameAllocator)
                        ↑
                   kernel_main() — direct frame alloc/free
```

```
MemoryObject (frozen) — standalone crate, no arch deps
    ├── lifecycle FSM (Created → Allocated → Mapped → Shared → Revoked)
    ├── virtual mapping slots (software-only, no MMU programming)
    ├── MemoryCapability + MemRights (stub, check() always returns true)
    ├── MemoryBackend trait (allocate/deallocate/properties)
    ├── MemoryResourceManager (backend registry, policy-based allocation)
    └── PlacementPolicy engine (scoring by latency/bandwidth/capacity/persistence)
```

### Gaps to Close

| Gap | Impact |
|-----|--------|
| `MemoryObject::map()` records `(vaddr, size)` in a software array but never programs page tables | No actual MMU connection |
| `MemoryResourceManager` stores backends as `*mut dyn MemoryBackend` (raw pointers) | Not compatible with SystemState ownership model |
| No `PmmBackend` adapter exists | MRM can't use `PmmBitmap` |
| `BootMemoryManager` duplicates MRM responsibilities | Two memory managers, unclear boundary |
| No runtime page table modification API | Can't map/unmap after `mmu_activate` |
| `kernel-memory-frozen` depends on `vivanta-boot-common` for `println!` | Should use kernel's own diagnostics |
| MRM is not referenced from `SystemState` | No resource management available at kernel_main |

## Decision

### 1. MRM Lives in `kernel/src/memory/`, Not a Separate Crate

The adapted MRM code becomes a module inside `vivanta-kernel`, not a standalone crate. Rationale:
- Tight coupling with kernel internals (PmmBitmap, SystemState, VMM) makes a separate crate boundary artificial
- Avoids circular dependencies (MRM needs PMM, PMM is in kernel)
- The frozen prototype served its purpose (validation); production code belongs in the kernel

The `kernel-memory-frozen/` crate is **retired** (not deleted — archived for reference). Its code is adapted, not imported as a dependency.

### 2. Three-Layer Memory Architecture

```
┌─────────────────────────────────────┐
│         MemoryResourceManager        │  ← resource orchestration
│  (policy engine, backend routing)    │
├─────────────────────────────────────┤
│            MemoryObject              │  ← logical memory regions
│  (lifecycle, virtual mappings, caps) │
├─────────────────────────────────────┤
│  PmmBackend │ OtherBackends (future) │  ← backend abstraction
│     ↓                                │
│  PmmBitmap (FrameAllocator)          │  ← physical frames
└─────────────────────────────────────┘
```

Each layer depends only on the one below:
- **PMM layer**: `PmmBitmap` — frame bitmap, zero dependencies on MRM/VMM
- **Backend layer**: `PmmBackend: MemoryBackend` — wraps `PmmBitmap`, translates frame alloc/free to backend alloc/free
- **Object layer**: `MemoryObject` — lifecycle, virtual mapping records, capability
- **Orchestration layer**: `MemoryResourceManager` — registers backends, routes allocations via policy

### 3. `PmmBackend` — Bridge Between PMM and MRM

```rust
// kernel/src/memory/pmm_backend.rs
pub struct PmmBackend<'a> {
    pmm: &'a mut dyn FrameAllocator,
    properties: MemoryProperties,
}
impl MemoryBackend for PmmBackend<'_> {
    fn allocate(&mut self, size: u64, align: u64) -> Result<PhysAddr, AllocError>;
    fn deallocate(&mut self, addr: PhysAddr, size: u64);
    fn properties(&self) -> MemoryProperties;
    fn name(&self) -> &'static str;
}
```

`PmmBackend::allocate(n, align)` calls `pmm.alloc_frame()` repeatedly until `n` bytes are covered, respecting alignment. `deallocate` calls `free_frame()` for each page.

### 4. `BootMemoryManager` Responsibilities Merge into MRM

Current `BootMemoryManager`:
1. Init bitmap at known location
2. Reserve kernel image, DTB, bitmap itself
3. Print stats
4. Finish → return PmmBitmap

New approach: MRM is initialized during `SystemState::from_boot_info()`:
1. Create `MemoryResourceManager`
2. Register `PmmBackend` (wrapping a yet-uninitialized PmmBitmap)
3. MRM reserves kernel/DTB/bitmap regions using `reserve()` on the underlying backend
4. After construction, `SystemState::memory_manager()` provides access

The `BootMemoryManager::print_stats()` becomes `MRM::print_stats()`.

This eliminates the `BootMemoryManager` struct entirely.

### 5. `MemoryObject::map()` Programs Real Page Tables

The frozen prototype's `map()` only stores `(vaddr, size)` in a slot. In V2 it must also program the MMU.

New signature:
```rust
impl MemoryObject {
    pub fn map(&mut self, vaddr: u64, size: u64, pt: RootPageTable) -> Result<usize, ObjectError>;
    pub fn unmap(&mut self, slot: usize, pt: RootPageTable) -> Result<(), ObjectError>;
}
```

`map()` calls `mmu_map_object(pt, vaddr, backend_paddr, size, mmu_flags)`.
`unmap()` calls `mmu_unmap(pt, vaddr, size)`.

This requires a new arch-api runtime MMU writer.

### 6. New arch-api Runtime MMU Functions

Current `arch-api/src/boot/mmu.rs` has boot-only functions (called before `mmu_activate`). Add a new module for runtime page table modification:

```rust
// arch-api/src/mmu.rs (additions)
extern "Rust" {
    /// Map a physical region at a virtual address in a live address space.
    /// Used by MemoryObject::map() at runtime (after mmu_activate).
    pub fn mmu_map_object(pt: RootPageTable, vaddr: u64, paddr: u64, size: u64, flags: MappingFlags);

    /// Unmap a virtual region from a live address space.
    /// Used by MemoryObject::unmap().
    pub fn mmu_unmap(pt: RootPageTable, vaddr: u64, size: u64);
}
```

Implementation in `arch-aarch64`:
- Walk page table from `pt` (stored as physical address of root level)
- Create/update descriptors at leaf level with proper attributes (AP, XN, AF, SH)
- On unmap: invalidate descriptors, issue `TLBI VAAE1IS`, `DSB SY`, `ISB`

This mirrors the existing boot `mmu_map_range`/`mmu_map_ram` but operates on arbitrary live page tables with full attribute control.

### 7. `SystemState` Gains `memory_manager` Field

```rust
pub struct SystemState {
    identity: IdentityState,
    hardware: HardwareState,
    memory_manager: MemoryResourceManager<'static>,  // NEW
    is_initialized: AtomicBool,
}
```

Construction in `from_boot_info()`:
1. Read memory map from BootInfo
2. Locate usable region, compute bitmap location (as currently done in kernel_main)
3. Init PmmBitmap at bitmap_base
4. Wrap in PmmBackend
5. Register with MemoryResourceManager
6. Reserve kernel/DTB/bitmap via MRM

Getter:
```rust
impl SystemState {
    pub fn memory_manager(&self) -> &MemoryResourceManager<'static>;
    pub fn memory_manager_mut(&mut self) -> &mut MemoryResourceManager<'static>;
}
```

### 8. `MemoryResourceManager` Uses References, Not Raw Pointers

Current frozen code stores `*mut dyn MemoryBackend`. Change to `&'a mut dyn MemoryBackend`:

```rust
pub struct MemoryResourceManager<'a> {
    backends: [Option<&'a mut dyn MemoryBackend>; MAX_BACKENDS],
    count: usize,
    // ...
}
```

This eliminates unsafe pointer dereferences and aligns with SystemState ownership.

### 9. `MemoryObject` in Kernel State — Not in SystemState Directly

`MemoryObject` instances are not stored in SystemState. They are created on demand by the MRM and returned to callers (similar to how files are opened). The MRM tracks object creation via `next_object_id` but does not store objects — callers own them.

This is consistent with ADR-020 (System Runtime Ownership): SystemState owns the resource *manager*, not individual resources.

### 10. Stub Global Allocator to Real Allocator Migration

The current `StubAllocator` in `kernel/src/lib.rs` panics on any allocation. Once MRM is operational, a kernel heap allocator backed by `PmmBackend::allocate()` can be implemented. This is **deferred** — V2/M5 keeps the stub; the real allocator is V2.x follow-up.

## Consequences

### Positive
- Single memory management path from boot to runtime (BootMemoryManager eliminated)
- MemoryObject lifecycle connected to real MMU for the first time
- Clean layering: PMM → Backend → MRM → SystemState
- PmmBitmap unchanged (existing FrameAllocator contract preserved)
- Raw pointer elimination in MRM improves safety
- No new crate boundaries (MRM lives in kernel)
- Runtime MMU API enables future VMM features (page faults, COW, demand paging)

### Negative
- `kernel/src/memory/` module added (~400 lines adapted + ~100 new)
- Two new arch-api extern functions (aarch64 implementation ~80 lines)
- `SystemState::from_boot_info()` grows memory init logic (~30 lines)
- `MemoryObject::map()` dependency on `RootPageTable` couples object layer to VMM
- Real global allocator deferred (stub panics remain for V2/M5)

### Risk Mitigation
- Frozen prototype archived, not deleted — easy to reference original logic
- BootMemoryManager removal is safe: MRM's reserve() wraps same PmmBitmap::reserve()
- Runtime MMU functions mirror existing boot functions — proven pattern
- QEMU validation is sufficient for initial MRM testing (no real HW needed)
- MemoryObject mapping validated by M3-BC experiment — logic is tested

## Implementation Phases

### Phase 1: ADR-025 (this document) — ratification

### Phase A: `PmmBackend` adapter (kernel/src/memory/pmm_backend.rs)
- Implement `MemoryBackend` for `PmmBackend<'a>`
- `allocate()` → repeated `alloc_frame()`, alignment handling
- `deallocate()` → repeated `free_frame()`
- Test: allocate some frames, verify via free_count

### Phase B: MRM in SystemState
- Adapt `MemoryResourceManager` with lifetime parameter, remove raw pointers
- Remove `vivanta-boot-common` dependency (use kernel's println or none)
- Add `memory_manager` field to SystemState
- Implement boot reservation in MRM (merge BootMemoryManager logic)
- Remove `BootMemoryManager` struct, update `kernel_main()`

### Phase C: Runtime page table writer (arch-api + arch-aarch64)
- Add `mmu_map_object()` and `mmu_unmap()` declarations to arch-api
- Implement on AArch64 (walk, descriptor write, TLBI)
- Implement on arch-test-stub (no-op or mock)

### Phase D: `MemoryObject::map()` → MMU
- Add `RootPageTable` parameter to `MemoryObject::map()` and `unmap()`
- Call `mmu_map_object()` / `mmu_unmap()` in implementations
- Connect `MemRights` to `MappingFlags` translation

### Phase E: Validation on QEMU
- Create a `MemoryObject` via MRM from kernel_main
- Map it into kernel address space via `map()`
- Write a test pattern, read it back
- Unmap and verify fault on access
- Print MRM stats