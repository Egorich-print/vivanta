# RFC-012: Memory Object — Architectural Experiment

## Status

**Frozen** (per ADR-011).

Design preserved for future revival. Implementation blocked until VMM, page
fault handling, and userspace exist.

## Original implementation

`kernel/src/memory/object.rs` — MemoryObject state machine with:
- Lifecycle: Created → Allocated → Mapped → Shared → Revoked
- Clone (shared backend storage, no COW)
- Share (capability-based grant to another owner)
- Revoke (terminates all mappings)
- Virtual mapping slots (max 4 per object)
- ObjectError for invalid transitions

## Motivation (preserved)

PMM allocates frames but higher layers need:
- Lifecycle tracking (who owns what)
- Virtual mapping management (which pages map where)
- Sharing (DMA buffers, IPC, device memory)
- Revocation (hot-unplug, driver reset)
- Isolation (capability-gated access)

PMM alone is insufficient for these requirements. A higher-level abstraction
over physical allocations is necessary.

## Preconditions for revival

1. VMM exists and is stable (Stage 5)
2. Page fault handling exists and is tested (Stage 2 + Stage 5)
3. Userspace exists (Stage 6)
4. At least two independent use-cases for MemoryObject exist

## Design questions for revival

1. Should MemoryObject own physical pages directly, or reference a VMO-like
   intermediate?
2. Is clone/share revocable or fire-and-forget?
3. Should mappings be tracked in the object or in the address space?
4. Is MemoryObject a kernel resource or a library abstraction?
5. Should revoke cascade to derived objects?

## Related RFCs

- RFC-013 (Capability System) — MemoryCapability provides access control
- RFC-014 (Hardware Graph) — DeviceObject extends same pattern to devices
- RFC-015 (Tiered Memory) — MemoryBackend provides physical storage
- RFC-016 (Placement Policy) — selects backend for allocation
