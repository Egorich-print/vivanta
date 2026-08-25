# ADR-031: Capability-Based Distributed Identity and Cluster Membership

## Status: Proposed
## Date: 2026-08-09

## Decision

Cluster nodes are identified by **Ed25519 node keypairs** (persistent once a
storage driver exists; volatile per-boot before that). Device identity is a
node-signed claim. Workload identity is `node_pk || uuid`. Cluster membership is
a **signed membership document** replicated across nodes, with an epoch for
revocation. All fabric messages are authenticated by the sender's node key.

## Context

Vivanta already validates Ed25519 keygen/sign/verify (M1-A) and separates
boot/runtime/persistent identity (ADR-024). The distributed design must extend
identity without changing the kernel. Using the validated identity primitives
for node keys keeps one identity model and gives authenticity at the edge with
no PKI. Membership must be revocable and must not create a globally trusted
root: a node is authoritative only over its own resources.

## Consequences

Positive: reuse of validated code; authenticated fabric by default; zero PKI
dependencies; TTL-based revocation is cheap at home-cluster scale.
Negative: per-message signing has overhead (mitigated by deriving per-pair
session keys after handshake); volatile pre-storage identity means nodes
re-key on reboot until M7.
Alternatives considered: x509 certificates (PKI overhead, no benefit),
hostname-based ID (forgeable), per-cluster CA (single point of trust).

## Related

RFC-001 identity model; ADR-024 identity separation; roadmap M7/M8. See
`docs/distributed/VIVANTA-DISTRIBUTED-ARCHITECTURE.md` §2.1.