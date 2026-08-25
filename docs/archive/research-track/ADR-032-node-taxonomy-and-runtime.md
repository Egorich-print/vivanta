# ADR-032: Node Runtime and Node Taxonomy (Native / Linux Agent / External Accelerator)

## Status: Proposed
## Date: 2026-08-09

## Decision

Vivanta clusters consist of three node kinds, all speaking the same fabric
protocol:

1. **Native Vivanta node** — runs the Vivanta kernel; node runtime in userspace.
2. **Managed Linux node** — Linux + a Rust `vivanta-agent` daemon exposing the
   host's CPUs, GPUs, NPUs, TPUs to the fabric (e.g. AIBOX-1684X, GPU boxes,
   Poco F1-class phones).
3. **External accelerator** — a device reachable only through an owning node's
   backend; never a cluster member itself.

## Context

The research shows the shortest path to a real heterogeneous cluster is the
transitional architecture: the fabric protocol is identical on native nodes and
agents, so Vivanta can control BM1684X/GPU/NPU hardware long before Vivanta has
kernel drivers for them. This honors "services over kernel features" and the
mechanism/policy split.

## Consequences

Positive: heterogeneous control within months; protocol validated before kernel
networking is complete; vendor isolation (BModel/CUDA etc. stay userspace).
Negative: agent nodes depend on a host Linux; agent cannot enforce Vivanta's
memory isolation, so its guarantees are weaker (documented as such).
Alternatives: waiting for native GPU/TPU drivers (years), or a second wire
protocol for agents (rejected — one protocol everywhere).

## Related

`VIVANTA-DISTRIBUTED-ARCHITECTURE.md` §4; roadmap M8/M13/M16; ADR-039.