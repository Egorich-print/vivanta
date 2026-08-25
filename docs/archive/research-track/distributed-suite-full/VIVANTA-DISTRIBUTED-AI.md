# Vivanta — Distributed AI Architecture

> **Status:** Proposed — deliverable 4.
> **Date:** 2026-08-09.
> **Companion docs:** `VIVANTA-DISTRIBUTED-ARCHITECTURE.md`,
> `VIVANTA-HETEROGENEOUS-COMPUTE.md` (accelerator API),
> `VIVANTA-ROADMAP.md` (M12–M17 milestones).

---

## 1. Thesis

> **Live local first, distribute only when a node can't fit the model, and
> treat distributed execution as layer/pipeline sharing, not as a new
> programming model.**

Evidence base:
- llama.cpp RPC ("distribute when you can't fit"; each remote hop pays a round
  trip) — KNOWN (observed by llama.cpp community).
- vLLM PagedAttention (KV-cache memory management is the real serving
  bottleneck at small scale) — KNOWN (published).
- Decode is memory-bandwidth-bound, prefill is compute-bound — KNOWN (LLM
  serving literature). This asymmetry is *the* design lever for a
  heterogeneous cluster.

---

## 2. Levels of distributed inference

| Level | Description | Verdict for us | When |
|---|---|---|---|
| L1 | One model, one node | ✅ Base case | M12 |
| L2 | Multiple independent models across nodes | ✅ Default cluster mode | M12–13 |
| L3 | Replicas + load balancing | ✅ Simple, high value | M13–14 |
| L4 | Pipeline-parallel inference | ⚠️ Marginal; proven by llama.cpp-RPC | M15 (experiment) |
| L5 | Tensor-parallel inference | ❌ Not on 1 GbE/phone | M17+ |
| L6 | MoE expert distribution | 🔬 Research | M17+ |
| L7 | Disaggregated prefill/decode | 🔬 Research (attractive for BM1684X) | M17+ |

**Rationale:** L5 (tensor-parallel) requires all-reduce per token over fast
fabric (NVLink/NCCL-grade). Home/edge Wi-Fi/1GbE is exactly the wrong case.
L4 (pipeline) works when the data volume between stages is small and the
communication is latency-tolerant — with a small number of large layers across
few nodes, that's acceptable. It is also **exactly what llama.cpp-RPC already
does**, so it is the least-risk way to ship real distributed LLM inference to
prove the fabric the honest way.

---

## 3. Local inference architecture (M12)

```
model package (vivanta-model)
   → backend selection (vivanta-accelerator)
   → run (vivanta-ai-runtime -> ggml/llama / ONNX Runtime)
   → tokens
```

Implementation candidates (RESEARCH HYPOTHESIS to be benchmarked):
- **ggml/llama.cpp** — best library support for GGUF, CPU + Metal + CUDA, has
  existing RPC backend to learn from.
- **ONNX Runtime** (via `ort` Rust crate) — graph + CPU/GPU, more general than
  LLM, good for benchmarks and CV.
- Both are *engines*, exactly interchangeable behind the accelerator channel.

Local ensemble: embed ggml as the first native CPU + Metal backend (the softest
to get working and the most relevant to Mac).

---

## 4. The model package and layers

```
vivanta-model (content-addressed)
   ├── metadata (io spec, context window, temperature default)
   ├── graph option (ONNX/StableHLO interchange; may be null if black-box)
   ├── weights (SafeTensors | GGUF)
   ├── tokenizer (tokenizers JSON / HF tokenizer)
   ├── memory (min int8 / KV requirement)
   ├── capabilities
   └── artifacts[]:  backend → { sha256, kind, blob path }
         bm1684x: BModel
         cuda:   .so/.cubin
         vulkan: SPIR-V
         metal:  pipeline
         cpu:    GGUF/ONNX
```

- Content hash = model identity; signature verified at load.
- `CAS` caches by hash, nearest-capable-node pattern (see research §12.2).

---

## 5. Placement of a request (pseudo-flow)

```mermaid
flowchart TB
    req[model request with context]
    req --> P[Orchestrator: which backend has this model?]
    P -->|resident on N2 (BModel)| N2[Node N2: ARM A53 + BM1684X TPU]
    P -->|not resident| D[select node via score]
    D --> L[transfer artifact via CAS + bulk]
    L --> N1[Run; KV kept on decode node]
    N1 --> R[Batching, prefill]
    ...
```

Actors:
- **Orchestrator** decides route only.
- Acceleration: model levels exact. "Good model" if resident on node where
  needed.

The KV cache lives **on the decode node**; prefill on a compute-rich node
(BM1684X) and decode on a memory-bandwidth-rich node (GPU/CPU) is the L7
design for M17+.

---

## 6. Distributed runtime behaviors

- **Idempotent summary**: the `(workload_id, op_seq)` covering dedup.
- **Retry on node loss** (watchdog; re-place).
- **Checkpointing** for long  states (KV + weights) as part of the model
  registry; resume from CAS (in `vivanta-storage`).
- **Cancellation** propagation: parallel `cancel(cap_id)` to participating
  nodes.
- **Backpressure** on bulk channel between producer and consumer nodes.

---

## 7. Copy and  aggregate regions

Both copy tensors (weights, KV) **through** the backend's allocate/upload/
download ("remote buffer"), never through a hypothetical shared virtual
address space. The network channel is **bulk** for weights; **control** for
scheduling/accounting/telemetry.

```
Weights:   source CAS → landing node CAS → artifact load → device (one bulk)
KV cache:  stays on decode node; only copy if HQ-level migration or
           checkpointing.
Activations (L4): each stage ships output tensors to the next stage node via
           bulk (this data volume × #tokens is the "L4 bottleneck" measured by
           `vivanta-bench`).
```

---

## 8. What the heterogeneous cluster is good at

| Workload | Best node | Why |
|---|---|---|
| LLM decode (throughput) | GPU node / fast-CPU | memory-bandwidth-bound |
| LLM prefill | BM1684X / GPU | compute-bound; good batching |
| ASR | BM1684X BModel | compute-bound |
| Vision / CV | BM1684X / RK3568 | smaller models, fit |
| Embeddings / rerank | GPU / CPU batch | low latency |
| Tiny on-device copilots | phones / RK3568 | opportunistic, battery-gated |

This decisional example demonstrates that **heterogeneous is useful because
the optimal kind of silicon differs by workload class**.

---

## 9. Benchmark suite (gate for M15)

`vivanta-bench`:

| Metric | Meaning |
|---|---|
| startup | time to first ad. |
| model load | artifact→loaded overhead |
| tokens / s | sustained decode |
| first-token | prefill + decode onset |
| memory | peak, KV |
| network | est / os pages per request |
| transfer | weight bulk |
| schedule | score+dispatch time |
| migration | KV+cache re-home cost |
| recovery | node-loss→resume time |

**Break-even**: compare
"local (L1) vs remote (L2 route to a node that can hold it) vs distributed
(L4 layer-split)" — with the rule "distribution only if one node can't fit".
Numbers from this bench are **the hard gate for M15**.

---

## 9. Risks / unknowns

- AAA BM1684X memory headroom for 7B/13B in practice — UNKNOWN until box.
- L4 throughput over 1 GbE — UNKNOWN until bb.
- Phone WiFi quality — will reject via admission.
- Engine choice ggml vs ort for v1 — EXPERIMENTALLY VERIFIABLE at M12.

---

*Next: `VIVANTA-ROADMAP.md`.*