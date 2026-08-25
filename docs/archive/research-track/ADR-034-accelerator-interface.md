# ADR-034: Capability-Based Accelerator Interface (two-channel)

## Status: Proposed
## Date: 2026-08-09

## Decision

The accelerator abstraction is a **small common lifecycle (capability-aware)**
plus an **opaque-artifact channel** plus a **typed vendor-extension channel**:

- Common: `info / allocate / upload / execute / wait / release`.
- Execution is expressed as either *artifact-level* (whole-model: BModel, CUDA
  module, SPIR-V, ONNX) or *command-level* (fine ops) via `OpaqueWork`.
- The fabric never parses an artifact; it transports and gates it.
- `as_vendor()` downcast provides typed vendor hooks (e.g. `Bm1684xSpecific`).

## Context

The brief's concern is real: a closed `trait Accelerator` erases
hardware-specific optimization. CUDA, Vulkan, BMRuntime, WebGPU, llama.cpp all
show the correct pattern: a thin common lifecycle over subscription, with the
device-specific code staying typed in its own crate. This preserves both a
uniform dispatch mechanism and full vendor power.

## Consequences

Positive: uniform scheduler/dispatcher; raw performance preserved; easy to add
backends (CPU → Metal → CUDA → BM1684X → Vulkan); capability checks are uniform.
Negative: the common lifecycle cannot express everything (by design) — 
`as_vendor()` is an escape hatch that must be used carefully; two execution
modes add a small conceptual surface.
Alternatives: fully generic op IR (rejected: destroys performance); no common
API at all (rejected: no uniform fabric.)

## Related

`docs/distributed/VIVANTA-HETEROGENEOUS-COMPUTE.md` §3–4; roadmap M10/M11;
ADR-033.