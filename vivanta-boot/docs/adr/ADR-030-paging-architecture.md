# ADR-030: Paging Architecture — Mechanism and Policy

**Status:** Accepted  
**Date:** 2026-07-30  
**Related:** ADR-015 (Arch Boundary Contracts), ADR-019 (User Page Permissions), ADR-021 (BootInfo Escape Prevention)

---

## Context

Prior to this ADR, `arch-aarch64/src/paging.rs` combined page-table descriptor constants, walk logic, mapping operations, and type definitions in a single module. `arch-aarch64/src/mmu.rs` duplicated many of these constants and contained its own walker, split-block logic, and barrier/TLBI helpers.

This duplication made the MMU layer fragile:

- Two independent sets of descriptor constants (`DESC_*` vs `ENTRY_*`) with identical values
- Two walker implementations with different APIs
- No clear boundary between page-table mechanism (what the hardware mandates) and allocation policy (what the kernel decides)
- Risk of `mmu.rs` growing into a God object

## Decision

### 1. Mechanism vs Policy Split

The paging subsystem is divided into two layers:

**Mechanism (`paging/`)** — pure page-table manipulation:

- Knows nothing about allocation, frame management, or kernel policy
- Operates on raw physical addresses of tables
- Can be tested independently of any allocator
- Never calls `alloc_frame`, never touches `FrameAllocator`

**Policy (`mmu.rs`)** — allocation-aware construction:

- Imports from `paging/` for all table operations
- Owns `FrameAllocator`, `PageTableBuilder`, `PageTableGuard`
- Owns the runtime mmu entry points (`mmu_map_object`, `mmu_unmap`)
- Decides when to split blocks, when to create intermediate tables

### 2. `paging/` Module Structure

```
paging/
    mod.rs          — Core types: MemoryType, Permissions, MappingFlags, Mapping
    descriptor.rs   — VMSAv8-64 descriptor constants (Stage 1, 4 KB granule)
    walker.rs       — Table walk, split_block, read_desc/write_desc, barriers, TLBI
    mapper.rs       — PageTable handle: map, unmap, translate, map_region
```

### 3. Descriptor Constants

All AArch64 VMSAv8-64 descriptor constants live in `descriptor.rs` and use the `DESC_` prefix.

Groups:

| Group | Constants |
|-------|-----------|
| Validity | `DESC_VALID`, `DESC_TABLE` |
| Shareability | `DESC_SH_NON`, `DESC_SH_OUTER`, `DESC_SH_INNER` |
| Access | `DESC_AF` |
| Permissions | `DESC_AP_RW_EL1`, `DESC_AP_RO_EL1`, `DESC_AP_RW_EL0`, `DESC_AP_RO_EL0` |
| Execute | `DESC_PXN`, `DESC_XN` |
| AttrIndex | `DESC_ATTRIDX_NORMAL`, `DESC_ATTRIDX_DEVICE`, `DESC_ATTRIDX_NORMAL_NC` |
| Address | `ADDR_MASK`, `ADDR_MASK_BLOCK` |

### 4. Walker Invariants

`walker.rs` is pure mechanism. The `split_l2_block` function:

- Never allocates memory
- Only transforms descriptor bits
- Requires the caller to provide a valid, zeroed L3 table frame
- Executes the necessary barrier after updating the L2 entry

### 5. Mapper Invariants

`mapper.rs` (`PageTable` struct) provides map/unmap/translate on a pre-existing table hierarchy.

| Operation | Guarantee |
|-----------|-----------|
| `translate(va)` | Returns `Some(pa)` if a valid mapping exists, `None` otherwise. Never modifies tables. |
| `map(va, pa, flags)` | Writes a single L3 page or L2 block descriptor. Panics if intermediate tables don't exist. |
| `unmap(va)` | Clears an L3 page or L2 block descriptor. No-op if no mapping exists. |
| `map_region(va, pa, size, flags)` | Iterates `map()` with 2 MB block coalescing. Panics on missing intermediate tables. |

`PageTable` is not allocation-aware. Use `PageTableBuilder` (in `mmu.rs`) if allocation is required.

### 6. Bootstrap Flow

```
early_mmu.rs                  mmu.rs
    │                            │
    │  MAIR, TCR, SCTLR          │
    │  Identity map (2 MB)       │
    │  TTBR0 ← L1 phys addr      │
    │  MMU on                    │
    │                            │
    ▼                            │
boot.rs                         │
    │  mmu_init()                │
    │  → PageTableBuilder::new() │
    │  → allocates L1..L3        │
    │  → returns root PA         │
    │                            │
    ▼                            │
kernel_main                     │
    │  mmu_activate()            │
    │  → builder.finish()        │
    │  → guard.activate()        │
    │  → replaces identity map   │
    ▼                            ▼
Runtime (new page table)     Identity (archived)
```

### 7. Ownership

- `early_mmu` owns the identity page tables (static, never freed)
- `mmu.rs` owns the runtime `PageTableBuilder` and `PageTableGuard`
- `PageTableGuard::activate` writes TTBR0 and enables MMU with the new tables
- After activation, the identity tables are no longer reachable via TTBR0

### 8. Platform Rule

Platform crates (`platform-*`) describe only addresses, IRQs, and clock rates.
All UART driver logic lives in `boot_common::pl011`.

## Consequences

**Positive:**

- Duplicate constants eliminated. Single source of truth in `descriptor.rs`.
- Mechanism can be tested without an allocator or physical memory.
- `mmu.rs` is now clearly the policy layer — it decides when to allocate, split, and activate.
- Adding a new architecture (e.g., RISC‑V) follows the same split: an `arch-riscv64` with its own `paging/` and `mmu.rs`.

**Negative:**

- Some code paths now have more module indirection (import from `paging::walker` instead of local helpers).
- The split requires discipline: new MMU features must be clearly classified as mechanism or policy.

**Non-goals (deferred to V2.5+):**

- TTBR1 / ASID support
- Per-CPU page tables
- LPA (Large Physical Address)
- Stage 2 translation (virtualisation)

## References

- ADR-015 — Arch Boundary Contracts (MappingFlags at the kernel boundary)
- ADR-019 — User Page Permissions (PageFlags encoding)
- ADR-021 — BootInfo Escape Prevention (identity map lifecycle)
