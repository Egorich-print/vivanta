# Memory Architecture — Resource-Oriented Memory Model

## Overview

Vivanta does not treat memory as fixed categories (RAM, swap, VRAM, persistent memory). Instead, memory is a set of **heterogeneous resources** with different properties (latency, bandwidth, persistence, coherence).

The architecture is built around three layers:

```
Memory Placement Policy
        │
Memory Resource Manager
        │
Memory Object (architectural centre)
        │
        ├── VMM (address translation mechanism)
        ├── PMM (physical frame allocator, as MemoryBackend)
        └── Capability Layer (deferred enforcement)
```

## Key Concepts

### MemoryObject

The unit of memory ownership and management. A MemoryObject ties together:

- Which backend provides physical pages (resource_id)
- Whether and where it is mapped (VirtualMapping)
- Who has access (MemoryCapability)

Operations: `create`, `clone`, `map`, `unmap`, `revoke`.

### MemoryBackend

A trait implemented by each physical memory provider:

- `System RAM` — the bitmap-based PMM (existing code)
- Future: `CXL memory`, `VRAM`, `Persistent memory`

Each backend reports `MemoryProperties`:

| Property | Description |
|----------|-------------|
| `capacity` | Total capacity in bytes |
| `latency_ns` | Approximate access latency |
| `bandwidth_mb_s` | Approximate bandwidth |
| `persistence` | Volatile or Persistent |
| `coherence` | FullyCoherent, IoCoherent, NonCoherent |
| `reliability` | Server (ECC) or Consumer |

### MemoryResourceManager

Central registry of backends. Responsible for:

- Registering backends at boot (and hotplug in future)
- Selecting the best backend for a given `PlacementPolicy`
- Allocating MemoryObjects

### PlacementPolicy

Controls which backend is preferred:

| Policy | Selects |
|--------|---------|
| `Fastest` | Lowest latency backend |
| `Largest` | Largest capacity backend |
| `Persistent` | Non-volatile backend |
| `Balanced` | Compromise between latency and capacity |

### Capability Model

Capabilities provide access control without full enforcement (deferred).

```rust
MemoryCapability {
    id: CapabilityId,
    object: MemoryObjectId,
    rights: MemRights,
    owner: OwnerId,
}
```

## Placement Policy Engine (M3-B)

Allocation decisions are based on scored backend properties, not backend names.

### AllocationRequirements

```rust
AllocationRequirements {
    size: u64,
    align: u64,
    preferred_policy: PlacementPolicy,   // Fastest | Largest | Persistent | Balanced
    max_latency_ns: Option<u32>,         // hard filter
    min_bandwidth_mb_s: Option<u64>,     // hard filter
    require_persistence: Option<bool>,   // hard filter
}
```

### Scoring

Each backend property is scored 0-100 per dimension:

| Dimension | Near/HBM | DDR | CXL | Storage |
|-----------|---------|-----|-----|---------|
| Latency   | 100     | 60  | 20  | 0       |
| Bandwidth | 100     | 70  | 40  | 10      |
| Capacity  | 10      | 50  | 80  | 100     |
| Persist   | 0       | 0   | 100 | 100     |

Policy determines per-dimension weights:

| Policy       | Latency | Bandwidth | Capacity | Persistence |
|-------------|---------|-----------|----------|-------------|
| Fastest     | 60%     | 25%       | 10%      | 5%          |
| Largest     | 10%     | 10%       | 70%      | 10%         |
| Persistent  | 10%     | 10%       | 20%      | 60%         |
| Balanced    | 30%     | 25%       | 25%      | 20%         |

Hard filters (`max_latency_ns`, `min_bandwidth_mb_s`, `require_persistence`) disqualify backends before scoring.

```
AllocationRequirements
        │
        ▼
  ┌─────────────┐
  │ Hard filter │ ← disqualifies non-matching backends
  └──────┬──────┘
         ▼
  ┌──────────────┐
  │ Weighted     │ ← scores remaining backends
  │ scoring      │
  └──────┬───────┘
         ▼
  Best backend → MemoryObject
```

## Relationship with Existing Subsystems

```
memory/
    MemoryResourceManager ──── MemoryBackend trait
         │                          │
         │                    PmmMemoryBackend (adapter)
         │                          │
         ▼                          ▼
    MemoryObject ────────────    PMM bitmap
         │
         ▼
    VMM (map/unmap)
         │
         ▼
    PageTableBuilder (arch-specific)
```

- `memory/` sits **above** `mm/` — it orchestrates resources
- `mm/` (VMM, PMM) provides mechanisms, not policy
- `memory/` never depends on arch specifics

## Future Directions

### CXL Support

A CXL-attached memory device will be registered as a second `MemoryBackend` with:
- Higher latency
- Higher capacity
- Possibly persistent

The `MemoryResourceManager` will select it when the policy favours capacity over speed.

### VRAM / GPU Memory

GPU memory will be a `MemoryBackend` with non-coherent access and high GPU-side bandwidth.

### Memory Migration

When multiple backends exist, the MRM can migrate MemoryObjects between them based on access patterns.

## Lifecycle (M3-C)

```
Created ──► Allocated ──► Mapped ──► Shared
    │            │            │
    │            │            │
    └────────────┴────────────┴──► Revoked
```

| State | Meaning | Allowed |
|-------|---------|---------|
| `Created` | ID reserved, no storage | `mark_allocated`, `revoke` |
| `Allocated` | Backend has physical pages | `map`, `clone`, `share`, `revoke` |
| `Mapped` | At least one virtual mapping | `map`, `unmap`, `clone`, `share`, `revoke` |
| `Shared` | Exposed to another owner | same as Mapped |
| `Revoked` | Invalidated | none |

### Multiple mappings

```rust
let s1 = obj.map(0x1000_0000, 4096)?;   // slot 0
let s2 = obj.map(0x2000_0000, 4096)?;   // slot 1
obj.mapping_count();                      // 2
obj.unmap(s1)?;                           // remove slot 0
```

### Clone

Same backend storage, independent mappings (no CoW yet):

```rust
let clone = obj.clone(new_id, new_cap);
```

### Share

Grant access via capability:

```rust
let handle = obj.share(cap_for_other);
// handle.object_id == obj.id
```

### Revoke

Invalidates, clears mappings, future operations fail:

```rust
obj.revoke();
assert!(obj.map(addr, size).is_err());
```

## Files

```
kernel/src/memory/
    mod.rs          — module re-exports
    object.rs       — MemoryObject
    resource.rs     — MemoryBackend trait + MemoryProperties
    manager.rs      — MemoryResourceManager
    capability.rs   — MemoryCapability
    policy.rs       — PlacementPolicy
    pmm_adapter.rs  — PmmMemoryBackend (PMM as a backend)
```