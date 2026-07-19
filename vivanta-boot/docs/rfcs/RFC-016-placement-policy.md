# RFC-016: Placement Policy

## Status

**Frozen** (per ADR-011).

Design preserved for future revival. Implementation blocked until multiple
memory backends exist and real hardware property data is available.

## Original implementation

`kernel/src/memory/policy.rs` — Policy engine with:

- PlacementPolicy enum: Fastest / Largest / Persistent / Balanced
- AllocationRequirements struct: size, align, preferred_policy, optional
  constraints (max_latency_ns, min_bandwidth_mb_s, require_persistence)
- Score function: evaluates backend properties against requirements
- Dimension scores: latency, bandwidth, capacity, persistence (0-100 each)
- Weighted sum with per-policy weight vectors
- Hard filters: disqualify backends that violate constraints
- select_best(): returns index of highest-scoring backend

## Known issues

1. **No real data.** Scoring weights are arbitrary. They have never been
   calibrated against real hardware measurements.
2. **Single backend.** Only one backend ever exists (PmmMemoryBackend). The
   policy engine always returns the same answer.
3. **No test.** The scoring function has no unit tests.
4. **No dynamic adjustment.** Properties are fixed at boot. Real memory
   performance varies with workload, temperature, and power state.

## Motivation (preserved)

NUMA-aware and heterogeneous-memory systems need intelligent backend selection.
A scoring engine is the correct approach, but it cannot be designed or tested
without real hardware with multiple distinct memory tiers.

## Preconditions for revival

1. Multiple MemoryBackend implementations exist (RFC-015 prerequisite)
2. Real hardware with measurable memory tiers is available
3. A workload demonstrates suboptimal allocation with naive selection
4. The scoring weights can be derived from actual performance measurements

## Design questions for revival

1. Should weights be static (compiled in) or dynamic (calibrated at boot)?
2. Should the policy prefer deterministic allocation (same result every time)
   or adaptive (learn from workload patterns)?
3. Should applications influence policy via hints, or is policy purely kernel
   internal?

## Related RFCs

- RFC-012 (Memory Object) — policy selects the backend for MemoryObject
- RFC-015 (Tiered Memory) — MemoryBackend provides properties that policy
  evaluates
