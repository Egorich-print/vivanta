# RFC-010: Memory Resource Model

| Field | Value |
|-------|-------|
| **Status** | Draft |
| **Objective** | Formalise the resource-oriented memory architecture |
| **Depends on** | RFC-009 (Platform Capability Model) |

---

## 1. Motivation

RFC-009 defined how the kernel learns about platform capabilities at boot.
RFC-010 defines how the kernel **manages memory as a set of heterogeneous resources**,
not as a single flat pool of RAM.

Traditional OS memory model:

```
Physical RAM
    │
    ├── Kernel pages
    ├── User pages
    ├── Page cache
    └── Swap
```

Vivanta memory model:

```
AllocationRequirements (size, latency, bandwidth, persistence, policy)
    │
    ▼
Memory Resource Manager
    │
    ├── MemoryBackend: RAM    (latency=main,   persistence=volatile)
    ├── MemoryBackend: HBM    (latency=near,   persistence=volatile)   [future]
    ├── MemoryBackend: CXL    (latency=far,    persistence=persistent) [future]
    └── MemoryBackend: VRAM   (latency=near,   persistence=volatile)   [future]
        │
        ▼
    MemoryObject (shared, cloned, mapped, revoked)
        │
        ▼
    VMM / PMM (implementation mechanisms)
```

---

## 2. Architecture

### 2.1 MemoryBackend

A `MemoryBackend` is a provider of physical memory pages with known
properties. Each backend corresponds to one type of hardware resource.

```rust
pub trait MemoryBackend {
    fn allocate(&mut self, size: u64, align: u64) -> Result<PhysAddr, AllocError>;
    fn deallocate(&mut self, addr: PhysAddr, size: u64);
    fn properties(&self) -> MemoryProperties;
    fn name(&self) -> &'static str;
}
```

`MemoryProperties` captures:

- capacity, latency_ns, bandwidth_mb_s
- persistence (volatile / persistent)
- coherence (fully / IO / non-coherent)
- reliability (server-ECC / consumer)
- power class
- latency class (near / main / far / storage)
- bandwidth class (extreme / high / medium / low)

### 2.2 MemoryObject

The sole unit of memory ownership visible to the rest of the kernel.

```
MemoryObject
    ├── id, size
    ├── resource_id (which backend)
    ├── state (Created → Allocated → Mapped → Shared → Revoked)
    ├── mappings[] (multiple virtual mappings)
    └── capability (who can access)
```

Supported operations:

| Operation | Description |
|-----------|-------------|
| `create` | Reserve an ID, no storage yet |
| `mark_allocated` | Backend has allocated physical pages |
| `map(vaddr)` | Add a virtual mapping |
| `unmap(slot)` | Remove a virtual mapping |
| `clone()` | New object, same backend storage |
| `share(cap)` | Grant access to another owner |
| `revoke()` | Invalidate, clear mappings |

### 2.3 MemoryResourceManager

Central registry of memory backends. Selects the optimal backend for each
allocation request based on scored properties.

```rust
struct MemoryResourceManager {
    backends: Vec<(ResourceId, Box<dyn MemoryBackend>)>,
}
```

### 2.4 Policy Engine

Allocation requirements drive backend selection through three stages:

1. **Hard filters** — disqualify backends that cannot meet constraints
   (max latency, min bandwidth, required persistence, minimum capacity)
2. **Weighted scoring** — score each remaining backend 0-100 across four
   dimensions (latency, bandwidth, capacity, persistence)
3. **Selection** — pick highest-scoring backend

Policy weights:

| Policy | Latency | Bandwidth | Capacity | Persistence |
|--------|---------|-----------|----------|-------------|
| Fastest | 60% | 25% | 10% | 5% |
| Largest | 10% | 10% | 70% | 10% |
| Persistent | 10% | 10% | 20% | 60% |
| Balanced | 30% | 25% | 25% | 20% |

### 2.5 Capability Model

```rust
MemoryCapability {
    id: CapabilityId,
    object: MemoryObjectId,
    rights: MemRights { read, write, execute, map, share },
    owner: OwnerId,
}
```

Capability enforcement is **deferred** — the structure exists but all checks
currently return `true`. Full enforcement will be implemented when IPC and
userspace exist.

### 2.6 Relationship to VMM / PMM

```
memory/ (this RFC)
    │
    ▼  MemoryObject maps through VMM
mm/ (VMM, PMM)
    │
    ▼  PageTableBuilder handles arch-specific MMU
arch/
```

- VMM is a **mechanism**, not a policy layer
- PMM is adapters as a `MemoryBackend`
- `memory/` never imports arch-specific code

---

## 3. Lifecycle

```
Created ──► Allocated ──► Mapped ──► Shared
    │            │            │
    │            │            │
    └────────────┴────────────┴──► Revoked
```

Invalid transitions return `Err(ObjectError::InvalidTransition)`.

---

## 4. Backend Selection Flow

```
AllocationRequirements
    │
    ▼
Hard filters (latency, bandwidth, persistence, capacity)
    │
    ▼
Weighted scoring per policy
    │
    ▼
Best backend → MemoryObject allocated
```

---

## 5. Future Work

| Item | Description |
|------|-------------|
| Copy-on-write | `clone()` shares storage; CoW is deferred |
| Memory migration | Move objects between backends at runtime |
| Hotplug | Register/deregister backends at runtime |
| IOMMU integration | Device DMA via capability-granted MemoryObjects |
| Capability enforcement | Runtime checks against capability rights |
| Tier promotion/demotion | Policy-driven migration between fast/slow tiers |