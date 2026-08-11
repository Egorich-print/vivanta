# Architectural Decision Records

This directory indexes ADRs for the Vivanta project (formerly TheseusOS).

## Registry layout

- **Engineering ADRs (canonical):** `vivanta-boot/docs/adr/` — kernel/arch decisions.
- **Org-level / research ADRs:** `docs/adr/` — historical RFC-freeze records and
  the quarantined distributed-AI track (ADR-031…039).

## Active ADRs

| ADR | Title | Status | Location |
|-----|-------|--------|----------|
| ADR-001 | RFC Freeze | Accepted | `docs/adr/ADR-001-rfc-freeze.md` |
| ADR-002 | State Versioning | Accepted | `docs/adr/ADR-002-state-versioning.md` |
| ADR-011 | Phase Transition — Research → Engineering | Accepted (amended 2026-07-19) | `vivanta-boot/docs/adr/ADR-011-phase-transition.md` |
| ADR-013 | Privilege Transition Model EL1↔EL0 | Accepted | `vivanta-boot/docs/adr/ADR-013-privilege-transition.md` |
| ADR-014 | Architectural Boundaries | Accepted | `vivanta-boot/docs/adr/ADR-014-architectural-boundaries.md` |
| ADR-015 | Arch Boundary Contracts (`extern "Rust"`) | Accepted | `vivanta-boot/docs/adr/ADR-015-arch-boundary-contracts.md` |
| ADR-017 | Unified Execution Context | Accepted | `vivanta-boot/docs/adr/ADR-017-unified-execution-context.md` |
| ADR-018 | User Entry Transition Model | Accepted | `vivanta-boot/docs/adr/ADR-018-user-entry-transition.md` (canonical; see duplicates note) |
| ADR-019 | User Page Permissions and EL0 Memory Model | Proposed | `vivanta-boot/docs/adr/ADR-019-user-page-permissions.md` |
| ADR-020 | System Runtime Ownership | Accepted | `vivanta-boot/docs/adr/ADR-020-system-runtime-ownership.md` |
| ADR-021 | BootInfo Escape Prevention | Accepted | `vivanta-boot/docs/adr/ADR-021-bootinfo-escape-prevention.md` (canonical; see duplicates note) |
| ADR-022 | Minimal Driver Lifecycle Contract | Accepted | `vivanta-boot/docs/adr/ADR-022-driver-lifecycle-contract.md` |
| ADR-023 | IdentityState Model | Accepted | `vivanta-boot/docs/adr/ADR-023-identity-state-model.md` |
| ADR-024 | Identity Model Separation | Accepted | `docs/adr/ADR-024-identity-model-separation.md` |
| ADR-025 | Memory Resource Manager Integration | Proposed | `docs/adr/ADR-025-memory-resource-manager-integration.md` |
| ADR-030 | Paging Architecture — Mechanism and Policy | Accepted | `vivanta-boot/docs/adr/ADR-030-paging-architecture.md` |

## Quarantined (distributed / AI research track)

Proposed ADRs for the distributed heterogeneous-compute vision. **No
implementation exists; they do not describe kernel code.** Kept here for
research continuity, out of the M5.0 scope fence.

| ADR | Title | Location |
|-----|-------|----------|
| ADR-031 | Capability-Based Distributed Identity and Cluster Membership | `docs/adr/ADR-031-capability-distributed-identity.md` |
| ADR-032 | Node Runtime and Node Taxonomy | `docs/adr/ADR-032-node-taxonomy-and-runtime.md` |
| ADR-033 | Node and Resource Model | `docs/adr/ADR-033-node-resource-model.md` |
| ADR-034 | Capability-Based Accelerator Interface | `docs/adr/ADR-034-accelerator-interface.md` |
| ADR-035 | Workload Model and Execution Graph Ownership | `docs/adr/ADR-035-workload-model.md` |
| ADR-036 | Two-Level Heterogeneous Scheduler | `docs/adr/ADR-036-heterogeneous-scheduler.md` |
| ADR-037 | Model Package & Content-Addressed Storage | `docs/adr/ADR-037-model-artifact-package.md` |
| ADR-038 | Failure Semantics | `docs/adr/ADR-038-failure-semantics.md` |
| ADR-039 | Linux Agent & BM1684X Integration | `docs/adr/ADR-039-linux-agent-bm1684x.md` |

## Duplicate ADR numbers (to resolve)

| Number | Canonical | Duplicate |
|--------|-----------|-----------|
| ADR-018 | `vivanta-boot/docs/adr/ADR-018-user-entry-transition.md` | `docs/adr/ADR-018-user-entry-transition.md` (untracked, near-identical) |
| ADR-021 | `vivanta-boot/docs/adr/ADR-021-bootinfo-escape-prevention.md` | `docs/adr/ADR-021-system-state-encapsulation.md` (different decision: state encapsulation) |

The canonical file in `vivanta-boot/docs/adr/` wins for M5.0. The duplicate in
`docs/adr/` is historical and must be archived or reconciled, not re-read as an
authoritative decision.

## Deprecated ADRs

| ADR | Title | Superseded By |
|-----|-------|---------------|
| ADR-012 | Execution Model — ThreadContext vs ExceptionFrame | ADR-017 |

## ADR Template

Please use the `adr/template.md` file as a guide when creating new ADRs.
