# ADR-038: Failure Semantics (leases, idempotency, reconciliation)

## Status: Proposed
## Date: 2026-08-09

## Decision

The fabric assumes nodes fail. Semantics:

- **At-least-once execution with idempotent operations**: every fabric message
  carries `(workload_id, op_seq)`; receivers deduplicate retries.
- **Leases**: resource/capability grants have a TTL; on expiry they are
  reclaimed. Partition survivors hold valid leases.
- **Heartbeats + watchdog** on every node; on timeout → drain in-flight,
  re-place work, revoke capabilities, re-home from CAS checkpoint.
- **Node admission** handles thermal/battery/OOM; model load failures retry
  same-node then other-node; corrupted models refused by content-hash.
- **No quorum/raft in M8–M16**: single orchestrator with leased state;
  multi-head consensus only if the cluster grows past ~20 nodes.

## Context

Research (§16 of the brief, and llama.cpp-RPC's wholeness) shows the real
failures are node disappearance, partition, accelerator reset, OOM, and
corrupted models. At-least-once + idempotency + leases is the smallest correct
set for a home/edge cluster; exactly-once execute is impossible without
compute-and-checkpoint.

## Consequences

Positive: simple; robust on flaky WiFi/phones; no consensus complexity early.
Negative: duplicate compute possible on crash (bounded by lease + checkpoint
interval); single orchestrator is a single point of control (documented).
Alternatives: exactly-once with distributed transactions (complex, unsuitable),
no fault handling (unacceptable), full Raft membership (overkill).

## Related

`VIVANTA-DISTRIBUTED-AI.md` §6; roadmap M13/M15; ADR-031/036.