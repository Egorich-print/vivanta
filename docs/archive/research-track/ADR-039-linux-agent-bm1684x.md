# ADR-039: Linux Agent & BM1684X Integration (Transitional Architecture)

## Status: Proposed
## Date: 2026-08-09

## Decision

A Rust **`vivanta-agent`** runs on Linux hosts (including AIBOX-1684X) acting as
a Managed Linux node (ADR-032). It:

- enumerates host CPUs/GPUs/NPUs/TPUs into resource descriptors (ADR-033);
- exposes the accelerator lifecycle (ADR-034) for whatever the backends provide;
- runs `vivanta-ai-runtime` (llama.cpp / Metal / bmrt) for model execution;
- speaks the same fabric protocol (identity, leases, workload DAG, CAS).

**BM1684X specifics stay behind the vendor-extension channel of ADR-034.** The
BModel loader (`bmrt`, `libsophon`) and `as_bm1684x()` downcast live in the
`vivanta-bm1684x` crate only. Nothing in the fabric/CLI paths to CUDA knows the
BM1684X. The vendor SDK is vendored; no BM1684X silicon is required to build or
run the tree.

## Context

Transitional architecture means heterogeneous control without waiting for native
Vivanta kernel drivers. This ADR fixes the boundary: the agent is a normal
userspace driver, the BM1684X is delivered as a normal backend, and M
GPU/TPU backends follow the same recipe. It avoids the "agent becomes a second
kernel" trap by keeping the agent thin (lifecycle + descriptors), with engines
plugging in as backends.

## Consequences

Positive: heterogeneous cluster in months, vendor-isolated; validate the fabric
on Linux before the kernel has networking; reuse of existing llama.cpp/bmtooling;
cargo feature-gates `bm1684x` by real hardware.
Negative: agent inherits Linux trust/fault models (documented, weaker than kernel
isolation); needs C/Rust FFI shims for libsophon in the short term.
Alternatives: native kernel driver first (delays the goal by years), ship the
Chinese NPUs alone (rejected — GPU less useful later).

## Related

ADR-032 (node taxonomy), ADR-034 (accelerator interface), ADR-037 (model
package / BModel artifacts), `VIVANTA-HETEROGENEOUS-COMPUTE.md` §5, roadmap
M11/M13.