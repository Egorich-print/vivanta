# Session Summary — 2026-07-30

## Accomplished

### Paging Refactoring (7 commits)
1. Created `paging/descriptor.rs` — unified VMSAv8-64 descriptor constants
2. Migrated all consumers to `descriptor::*`
3. Extracted `paging/walker.rs` — walk_to_l3, split_block, TLBI, barriers
4. Extracted `paging/mapper.rs` — PageTable (map/unmap/translate/map_region)
5. PMM safety assert in `PmmBitmap::new()`
6. PL011 dedup — `boot_common::pl011::Pl011` replaces local `Pl011Uart`
7. Removed 4 archived boot_legacy crates from workspace

### MMU Smoke Tests
- `paging/self_test.rs` — descriptor constants, translate, readback
- Wired into boot flow via `mmu_self_test()` in arch-api

### ADR-030
- Paging Architecture — Mechanism vs Policy
- Documents the `paging/` (mechanism) vs `mmu.rs` (policy) split

### Integration Fixes (8 PRs)
1. **Scheduler**: `maybe_reschedule()` now performs actual context switch
2. **Identity**: Removed deprecated `state::identity` module
3. **MRM**: All allocation routes through `MemoryResourceManager`
4. **VMM**: `AddressSpace.map_pages()`, `unmap_pages()`, `query()`
5. **MemoryObject**: Delegates to `AddressSpace` instead of direct arch-api
6. **VMM backend**: Direct arch_api calls only in `address_space.rs`
7. **protect()**: Stub (deferred, requires arch-api `mmu_protect`)
8. **STATUS.md**: Updated to reflect current architecture

### Overflow Fix
- `resource.rs:79` — `(4u64 * 1024 * 1024 * 1024) as usize` for 32-bit targets

## Architecture State

```
Process
    │
Thread
    │
AddressSpace (map_pages, unmap_pages, query)
    │
MemoryObject (delegates to AddressSpace)
    │
VMM (arch_api::mmu)
    │
MRM (MemoryResourceManager)
    │
PMM (PmmBitmap)
```

### Invariants
- All physical pages allocated through MRM
- All map/unmap operations route through AddressSpace
- Direct `arch_api::mmu` calls only in `vmm/address_space.rs`
- Paging mechanism (`paging/`) has no allocation awareness

## Next Steps

1. **Scheduler v2**: priority, sleeping/blocked states
2. **Process model**: Task → Thread → AddressSpace binding
3. **EL0 / Userspace**: proper user-mode support
4. **IPC primitives**: message passing, shared memory
5. **protect()**: implement via arch-api `mmu_protect()`

## Commits This Session

```
d9c70e2 refactor(paging): split mechanism and policy, fix PL011 dedup, archive cleanup
e94cb76 test(mmu): add runtime smoke tests for paging layer
a853b33 docs: ADR-030 paging architecture — mechanism vs policy
(scheduler: wire timer-driven preemptive reschedule)
(identity: remove deprecated state::identity module)
(memory: route all allocation through MRM)
(vmm: add AddressSpace map_pages, unmap_pages, query API)
(memory: route MemoryObject map/unmap through AddressSpace)
(vmm: complete VMM backend integration)
(vmm: add protect() stub, update STATUS.md)
```

## Key Files Modified

- `arch-aarch64/src/paging/` — new directory (descriptor, walker, mapper, self_test)
- `arch-aarch64/src/mmu.rs` — policy layer only
- `arch-aarch64/src/early_mmu.rs` — uses descriptor::*
- `kernel/src/scheduler/mod.rs` — maybe_reschedule() with context switch
- `kernel/src/memory/object.rs` — delegates to AddressSpace
- `kernel/src/vmm/address_space.rs` — map_pages, unmap_pages, query, protect stub
- `kernel/src/lib.rs` — all allocation through MRM
- `kernel/src/state/identity.rs` — deleted (deprecated)
