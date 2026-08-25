# ADR-033: Node and Resource Model (Capability-Oriented Descriptors)

## Status: Proposed
## Date: 2026-08-09

## Decision

Distributed resources are described by **capability-oriented dynamic
descriptors** (a static `ResourceDescriptor` + a time-variant `ResourceState`),
not by a closed `ComputeResource` struct. Resources are addressed by capability;
availability is advertised and sampled; the kernel's internal `MemoryObject`
model is unchanged.

## Context

RFC-010/ADR-025 already model local memory as `MemoryBackend` + properties +
policy. The cluster needs the same idea extended to CPU/Memory/GPU/NPU/TPU/
Storage/Network across nodes. A capability-oriented model means "you hold the
GPU because you hold its capability with rights+TTL," which fits Vivanta's
capability lineage and prevents nodes from becoming universal roots. A closed
struct would fail to represent radically different silicon.

## Consequences

Positive: uniform cluster view; reuse of RFC-010 patterns; enables heterogeneous
cost-based placement; capability gating works from day one.
Negative: descriptors must be carefully versioned (wire ABI); dynamic state adds
traffic (bounded by sampling).
Alternatives considered: closed `ComputeResource` struct (rejected: can't
represent RK3568 NPU vs BM1684X vs GPU with one shape); pure "offers like
Mesos" without static descriptors (rejected: no basis for placement).

## Related

`VIVANTA-HETEROGENEOUS-COMPUTE.md` §2; roadmap M9; ADR-031.