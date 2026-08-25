# ADR-037: Model Package & Content-Addressed Storage

## Status: Proposed
## Date: 2026-08-09

## Decision

A **Vivanta Model Package** is the single interchange unit:

```text
metadata · graph (optional ONNX/StableHLO) · weights (SafeTensors|GGUF) ·
tokenizer · memory estimate · capabilities · artifacts[]: backend → blob
```

The **content hash of the package is the model identity**; the package is
signed. Artifacts (BModel, CUDA module, SPIR-V, ONNX, GGUF) are opaque blobs
transported and gated but never parsed by Vivanta. Storage is a
**content-addressed store (CAS)** with LRU caching + locality (nearest capable
node) via the fabric bulk channel.

## Context

Different hardware needs different artifacts of the same logical model. A
closed format would fail BM1684X (BModel), CUDA, and GGUF simultaneously. CAS
gives identity, dedup, caching, and provenance in one mechanism. The model
package keeps byte-artifact realities while giving the fabric a uniform
identity.

## Consequences

Positive: one pipeline for all backends; dedup across cluster; content
verification at load; artifact signatures.
Negative: A package can be large (multiple artifacts); the registry must track
multi-artifact packages.
Alternatives: single global format (rejected — violates vendor isolation);
vendor-specific only (rejected — no cross-backend).

## Related

`docs/distributed/VIVANTA-DISTRIBUTED-AI.md` §4; roadmap M7/M12; ADR-031.