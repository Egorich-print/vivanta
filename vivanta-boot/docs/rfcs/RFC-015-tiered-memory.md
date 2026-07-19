# RFC-015: Tiered Memory

## Status

**Frozen** (per ADR-011).

Design preserved for future revival. Implementation blocked until VMM exists
and multiple memory backends are available on real hardware.

## Original implementation

`kernel/src/memory/resource.rs` — MemoryBackend trait + MemoryProperties:

- PhysAddr, AllocError types
- LatencyClass: Near / Main / Far / Storage
- BandwidthClass: Extreme / High / Medium / Low
- PersistenceType: Volatile / Persistent
- CoherenceModel: FullyCoherent / IoCoherent / NonCoherent
- ReliabilityClass: Server / Consumer
- PowerClass: Low / Medium / High
- MemoryProperties aggregate struct
- MemoryBackend trait: allocate / deallocate / properties / name

`kernel/src/memory/pmm_adapter.rs` — PmmMemoryBackend:

- Wraps PmmBitmap as a MemoryBackend
- Allocates 1 frame per call (ignores size and alignment)
- Hardcoded DDR4-like properties (latency=80ns, bandwidth=25GB/s)

## Known issues

1. **Single backend only.** PmmMemoryBackend is the only implementation.
   CXL, VRAM, and persistent memory backends do not exist.
2. **Properties are hardcoded.** Latency, bandwidth, and capacity values are
   guesses, not measured from hardware.
3. **Allocate ignores size and alignment.** Always returns 1 frame. Multi-frame
   contiguous allocation is not implemented.
4. **No test on real hardware.** No tiered-memory platform is available.

## Motivation (preserved)

Heterogeneous memory (DDR4 + HBM + CXL + persistent memory) is becoming common
in server and edge platforms. Vivanta should support allocation from the best
backend for each workload. The MemoryBackend trait is the correct abstraction
for this, but it cannot be validated without hardware.

## Preconditions for revival

1. VMM exists (Stage 5)
2. At least two physically distinct memory backends exist on the test platform
   (e.g., RK3588 with DDR5 + SRAM, or QEMU with nvdimm simulation)
3. A use-case requires explicit backend selection (e.g., DMA buffer in
   coherent memory, persistent store in PMEM)

## Design questions for revival

1. Should backends be discoverable from FDT (e.g., `/memory` reserved regions)?
2. Should allocation be explicit (user picks backend) or automatic (policy
   decides)?
3. Should MemoryProperties be static (fixed at boot) or dynamic (bandwidth can
   change with workload)?

## Related RFCs

- RFC-012 (Memory Object) — MemoryBackend feeds physical pages to MemoryObject
- RFC-016 (Placement Policy) — scoring engine selects backend
