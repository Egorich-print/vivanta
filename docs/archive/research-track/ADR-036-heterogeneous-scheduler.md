# ADR-036: Two-Level Heterogeneous Scheduler (central placement + node admission)

## Status: Proposed
## Date: 2026-08-09

## Decision

Scheduling is **two-level**:

1. **Global placement** — the orchestrator picks node + backend for a workload
   using a scored cost function (compute fit hard-gate, memory fit, model
   residency, network cost, latency, power, queue depth, capability width).
2. **Node admission** — the node's runtime/agent accepts or rejects based on
   local truth (thermal, battery, load, capability, isolation). This is the
   Mesos-style offer/accept pattern.

Later, **intra-device queue scheduling** lives inside backends, never in the
fabric.

## Context

Centralized-only scheduling fails for heterogeneous nodes (the scheduler cannot
know what a phone can run today). Fully decentralized scheduling fails on
global placement and coordination. Two-level is the minimal architecture that
handles heterogeneity honestly, validated by Mesos/Nomad experience. The
score function is the brief's formula with `compute_fit` as a hard gate rather
than a soft term.

## Consequences

Positive: heterogeneous admission by the node that knows itself; central
placement keeps global objectives; simple at cluster scale (≤20 nodes).
Negative: single orchestrator is a coordination point (mitigated: leased state,
M17+ multi-head optional); scoring needs calibration.
Alternatives: work-stealing (complexity without need at this scale), auction
(latency, overhead), fully decentralized (lost global view).

## Related

`VIVANTA-HETEROGENEOUS-COMPUTE.md` §7; roadmap M14; ADR-038.