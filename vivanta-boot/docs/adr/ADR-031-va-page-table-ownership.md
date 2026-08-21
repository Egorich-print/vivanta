# ADR-031: Virtual Address Space and Page-Table Frame Ownership

## Status
Accepted

## Date
2026-08-21

## Related
ADR-030 (Paging Architecture — mechanism/policy split), ADR-019 (User Page
Permissions), mission reports 2026-08-21 (W^X enforcement; M5.1/M5.2 VM
lifecycle).

---

## Context

After M5/M6 the kernel had a working MMU path but no formal lifecycle for
virtual memory:

- User VAs were hand-picked constants; no allocator existed.
- `AddressSpace::protect()` was restricted to whole mappings.
- Page-table frames were allocated through `PageTableAllocator` with an
  **intentional-leak** lifetime: nobody recorded who owned a table frame,
  so none could ever be safely returned to the PMM. Worse, the original
  `MrmPageTableAllocator` dropped the backing `MemoryObject` immediately,
  returning live table frames to the PMM while descriptors still pointed
  into them (fixed in `df5702b`; latent since the M5 runtime mapper).
- `mmu_map_object` could not create missing intermediate tables, so
  allocator-chosen VAs outside boot-mapped regions were unmappable.

## Decisions

### 1. VA allocator (`vivanta-vm` crate)

A pure, hardware-free first-fit interval allocator:

- Domain-based: user allocations come from `[USER_VA_BASE=0x0100_0000,
  USER_VA_END=0x4000_0000)`; page 0 is a permanent guard region; kernel
  identity RAM/MMIO lies above the domain and is never handed out.
- The free list is the single source of truth (allocated ⇔ in-domain and
  disjoint from every free interval) — overlap is impossible by
  construction; frees that would overlap a free interval are rejected
  (`DoubleFree`), making double-free/foreign-free deterministic errors.
- All arithmetic overflow-checked; failures mutate nothing.
- Free intervals are kept sorted and merged (canonical form), proven by a
  model-checked 20 000-operation host stress test plus targeted invariant
  tests. Capacity (256 intervals) exhaustion is a deterministic error,
  never silent corruption.

The kernel AS uses a *disabled* allocator: its translations are
boot-identity-mapped and not allocator-managed.

### 2. Page-table frame ownership (registry model)

Chosen over refcounting and arenas because Vivanta is single-core, has no
shared page tables, and needs the simplest provably-correct model:

- The **architecture layer** notifies ownership transfers: every time a
  child table is installed (block split or missing-intermediate creation),
  `PageTableAllocator::table_installed(frame, parent, index, level)` fires.
- The **kernel** records installs in a global registry
  (`vmm::tables`): `{frame, as_id, level, parent_table, parent_index,
  backend}`.
- **Root frames are never registered** — they are boot-allocated and leak
  by design for the lifetime of their address space. Reclamation of roots
  is structurally impossible (no code path can reach a root through the
  registry).
- Registry exhaustion degrades to the intentional leak (deterministic,
  safe). Boot-era tables are unknown to the registry and therefore never
  reclaimed.

### 3. Reclamation preconditions (all mechanical, all enforced)

A table frame leaves the hierarchy only when:

1. **Empty by hardware truth**: `mmu_table_valid_leaves(frame) == 0`.
   Emptiness is read from the table itself, never inferred from the
   software shadow — split-inherited block pages make shadow-empty tables
   non-empty (this distinction is regression-tested).
2. **Unlinked**: the parent descriptor is cleared
   (`mmu_clear_table_entry`) *after* the emptiness proof, under the IRQ
   guard (single-core TOCTOU rule: a preempting context mapping into the
   AS between proof and unlink would resurrect a reachable table).
3. **Translation-inert**: every leaf translation under the table was
   already invalidated by per-page TLBI at unmap time; the cleared parent
   entry adds no translation.
4. **Registry-deleted**: ownership returns via
   `reclaim_page_table_frame`, which deallocates through the backend that
   owns the storage.

Reclamation loops to fixpoint (emptying an L3 may empty its L2). It runs
only from the explicit unmap/unregister paths — there is no background
reclaimer.

### 4. Range semantics for protect/unmap

`protect()` and `unmap_pages()` accept arbitrary page-aligned sub-ranges:

- Coverage must be complete (no gaps) or the call fails before mutating.
- Partially overlapping shadow mappings are split (head / covered / tail)
  so `MappingSet` remains an exact image of the hardware; slot capacity is
  pre-checked transactionally.
- Hardware is programmed once for the whole range; shadow commits last.

### 5. Aliasing policy

Virtual aliases (two VAs → one PA) are permitted at the mapping level;
physical-frame ownership always stays with the allocator's client
(MemoryObject / caller). The VMM never frees physical frames — unmap only
removes translations and reclaims *table* frames. Therefore
"unmap(A) frees P while B still maps P" is structurally impossible;
regression-tested in QEMU (alias unmap leaves the original translation
and PMM accounting untouched). COW-style shared ownership remains future
work and must revisit this section.

### 6. TLB obligations

| operation | invalidation |
|-----------|--------------|
| map (new leaf) | `tlbi_range` after descriptor write |
| block split | `tlbi_range` (in map/unmap/protect paths) |
| protect | `tlbi_range` after rewrites |
| unmap | `tlbi_range` after clears |
| table unlink | none required — subtree already uninstantiated by (3); parent entry adds no translation |

Known limitation: QEMU does not re-walk on permission-miss the way real
silicon caches entries, so a *missing* TLBI after permission widening is
not observable under emulation (mission-2 mutation M4). Narrowing-side
staleness is bounded architecturally by `tlbi_all_sync` on every
address-space switch.

## Consequences

**Positive:** virtual memory now has a closed ownership story — every
table frame is either (a) registry-tracked and reclaimable under proven
preconditions, or (b) intentionally leaked by explicit rule (boot-era,
registry overflow, roots). The mmap/munmap/mprotect syscall layer can be
built directly on `reserve/map_new_range/protect/unmap_range` without
changing the ownership model.

**Negative:** the registry adds ~40 B per table frame (bounded at 256
frames); reclamation is O(registry) per unmap; shared page tables or COW
will require extending the model (refcounts or ownership transfer), which
this ADR deliberately does not pre-design.

## Verification summary

- Host (`vivanta-vm`): 7 tests incl. 20k-op model-checked stress.
- QEMU: VM lifecycle test (map → partial protect → split-shadow check →
  restore → alias → unmap+reclaim with PMM delta assert → block-split
  non-reclamation → remap-with-registry assertion → teardown); full
  M5/M6/W^X/fault-scenario suite unchanged; 95 s stress clean.
- Mutations: M1 free-active-frame ✅ caught (PMM delta), M2 reclaim
  non-empty ✅ caught (walk panic), M3 skip unlink ✅ caught (registry
  assertion on remap), M4 overlap-check corruption ✅ caught (host test),
  M6 split-attribute loss ✅ caught (neighbor AF audit / scenario failure),
  M8 domain-boundary violation ✅ caught (host test); M5 TLBI = known
  limitation; M7 alias-free = structurally excluded; M9 root reclamation =
  structurally impossible.
