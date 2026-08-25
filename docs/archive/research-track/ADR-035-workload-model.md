# ADR-035: Workload Model and Execution Graph Ownership

## Status: Proposed
## Date: 2026-08-09

## Decision

A workload is a **capability-notated identity** plus a **serializable
Execution Graph (DAG)**. Vivanta owns the Workload identity, the graph spec,
partitioning, placement, migration, retries, checkpointing, backpressure,
cancellation, partial failure, and accounting. Vendor/compiler ownership begins
below the operator level (what an operator is, how it runs, hardware
instructions).

The kernel is not involved: workloads are user-space objects.

## Context

The hierarchy Application → Workload → ExecutionGraph → Operators → Device
Kernels → Hardware Instructions (research §7) is confirmed; the important
boundary is between ExecutionGraph and Operators. Placing this boundary in
user space mirrors the mechanism/policy split: the graph mechanism and the
operator policy are separable.

## Consequences

Positive: graph is a portable/serializable data artifact; new engines drop in
by implementing the operator layer; the kernel stays untouched.
Negative: the fabric must maintain placement/migration/fault state (complexity in
`vivanta-orchestrator` and per-node `vivanta-runtime`).
Alternatives: kernel-owned DAG (rejected — kernel becomes AI framework);
stateless dispatcher (migration); prevented faults néthent support orderly).

## Related

`VIVANTA-DISTRIBUTED-AI.md` §2; roadmap M13/M15; ADR-038 (failure semantics).