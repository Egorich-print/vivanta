# Vivanta — Roadmap (Distributed & Heterogeneous Compute)

> **Status:** Proposed — deliverable 5.
> **Date:** 2026-08-09.
> **Supersedes-for-this-track:** the distributed/AI portions of
> `docs/architecture/master-roadmap.md`. The kernel-track M1–M5 history stays
> authoritative; this roadmap is the **fabric + AI track**.
> **Companion:** research (`VIVANTA-DISTRIBUTED-ARCHITECTURE-RESEARCH.md`),
> architecture (`VIVANTA-DISTRIBUTED-ARCHITECTURE.md`), heterogeneous compute
> (`VIVANTA-HETEROGENEOUS-COMPUTE.md`), distributed AI
> (`VIVANTA-DISTRIBUTED-AI.md`) in `docs/distributed/`; ADRs in `docs/adr/`
> (031 identity · 032 node taxonomy · 033 resource model · 034 accelerator
> interface · 035 workload model · 036 scheduler · 037 model package · 038
> failure semantics · 039 linux agent/BM1684X).

---

## 0. How to read this roadmap

Every milestone defines:

- **Objective** — capability that becomes possible.
- **Dependencies** — what must exist already.
- **Implementation** — what code/components land.
- **Tests** — what proves it works.
- **Failure criteria** — when the design should be reconsidered.
- **Exit criteria** — what objectively means done.

Acceptance criteria are **observable**, not aspirational.

---

## 1. Milestone overview

```text
Kernel track (existing, continues)          Fabric + AI track (new)
────────────────────────────────────────    ────────────────────────────────
M1–M5  ✅ (kernel foundations done)          M8   Cluster identity + discovery
M6     System services (IPC, sockets)  ──►  M8   (needs sockets)
M7     Storage (persistent identity, CAS) ─► M8   (needs persistence)
M9     Resource model (descriptors)
M10    Accelerator API
M11    First accelerator backend (CPU/ggml)
M12    AI runtime (local model)
M13    Distributed runtime (remote exec)
M14    Heterogeneous scheduler (placement)
M15    Distributed AI (replicas / pipeline)
M16    BM1684X integration (AIBOX agent)
M17+   Advanced (KV disaggregation, MoE, tensor-parallel research)
```

Note on ordering: M8 (fabric basics) can start in parallel with M6/M7 kernel
work *as a pure userspace/agent-only experiment on Linux*, because the fabric
protocol does not need the kernel networking stack. This is the single biggest
reason the transitional architecture (§20) is the shortest path to a real
cluster.

---

## 2. Milestone definitions

### M6 — System services (kernel: IPC + sockets + userspace)

**Objective:** Userspace processes can communicate (IPC), and a node can talk to
the network — the precondition for *everything* fabric.

**Dependencies:** M4.5 (EL0), M5 (MRM).

**Implementation:**
- IPC: message passing channel + shared memory objects (MemoryObject share).
- Minimal sockets / network API in userspace (or a very thin syscall layer).
- First userspace services (LoggingService per existing roadmap; a `fabricd`
  stub).

**Tests:**
- Two EL0 processes exchange a message through IPC.
- A userspace daemon binds, listens, accepts on a socket.

**Failure criteria:** If IPC/sockets cannot be made safe without changing the
execution model, reconsider (ADR for IPC).

**Exit:** IPC round-trip demo on QEMU + RK3568; socket connect/accept demo.

---

### M7 — Storage + persistent identity + CAS

**Objective:** Nodes can persist state; node identity survives reboot; model
artifacts can be stored content-addressed.

**Dependencies:** M6 (IPC/sockets), device graph (ADR-022), storage driver.

**Implementation:**
- Storage driver (SPI NAND/eMMC) for native nodes.
- `vivanta-storage`: content-addressed store (blake3/sha256) + LRU cache.
- Persistent node identity: store Ed25519 keypair via storage (unblock ADR-024).
- `vivanta-model`: model package manifest + artifact hash.

**Tests:**
- Write → read → hash-verify a blob across reboot (identity survives).
- Two nodes' CAS deduplicate the same model package.

**Failure criteria:** If storage driver proves untenable on RK3568, fall back to
agent-only persistent identity (Linux filesystem) and keep native storage later.

**Exit:** Node keypair survives reboot; a model artifact is stored by hash and
fetched by hash.

---

### M8 — Cluster identity, discovery, membership (Fabric core)

**Objective:** A cluster forms: nodes authenticate, advertise, and join a
signed membership.

**Dependencies:** M6 (sockets), M7 (persistent identity), `vivanta-core`
(capabilities).

**Implementation:**
- `vivanta-fabric`: wire protocol messages + framing + Noise-style handshake.
- `vivanta-cluster`: membership doc, epochs, static bootstrap (`fabric.toml`),
  mDNS discovery (phase 2).
- `vivanta-runtime` (native) + `vivanta-agent` (Linux): the two node daemons.
- First capability issuance: node → resource capability with TTL.

**Tests:**
- Two QEMU instances + one Linux host form a 3-node cluster over TCP.
- A node impersonation attempt is rejected (signature mismatch).
- Revocation: node leaves → its capabilities expire; no stale access.

**Failure criteria:** If capability TTL/signature overhead proves too costly for
the small control messages, simplify (raw TCP + per-message sign only, no
handshake).

**Exit:** 3-node cluster, signed membership, node-to-node authenticated call.

---

### M9 — Resource model (descriptors + cost)

**Objective:** The fabric sees a truthful, queryable view of each node's
resources and accelerators.

**Dependencies:** M8 (membership), ADR-022 device graph.

**Implementation:**
- `vivanta-resource`: `ResourceDescriptor`, `ResourceState`, `CostModel`,
  memory hierarchy attrs.
- Nodes advertise descriptors at join and push state (load, thermal, battery).
- Orchestrator builds and serves a `ClusterView` query API.

**Tests:**
- A node advertises CPU/memory/BM1684X-descriptor; orchestrator queries it.
- State updates (load changes) propagate within a bounded time.

**Exit:** ClusterView is queryable and accurate for ≥3 heterogeneous nodes.

---

### M10 — Accelerator API (capability-gated)

**Objective:** The accelerator abstraction exists and is capability-enforced.

**Dependencies:** M9 (descriptors), `vivanta-core` capabilities.

**Implementation:**
- `vivanta-accelerator`: `Accelerator` trait + registry + artifact channel +
  vendor-extension seam.
- Capability checks on allocate/upload/execute/release.
- Test stub backend (no device) for unit/integration tests.

**Tests:**
- Capability-less caller is denied; capability holder succeeds.
- Opaque artifact round-trips without being parsed.

**Exit:** API test suite green; capability enforcement proven in tests.

---

### M11 — First accelerator backend: CPU (ggml/llama) — *deliberately not BM1684X*

**Objective:** A real workload runs through the accelerator API on CPU.

**Why CPU first (NOT BM1684X):** it needs no vendor dependency, no new
hardware, exercises the whole stack (package → backend → execute), and gives
the benchmark baseline. Per Vivanta's "no abstraction before second
implementation," the second backend (Metal) then validates the trait.

**Dependencies:** M10, `vivanta-model`.

**Implementation:**
- `vivanta-accelerator-cpu`: ggml/llama.cpp (or ONNX Runtime CPU) backend.
- Load a small GGUF model via the model package; run a prompt.

**Tests:**
- `vivanta-bench`: tokens/s, first-token latency, load time, memory.
- Second backend (Metal) runs same package with identical API.

**Exit:** Same model package runs on CPU and Metal through identical code paths;
bench numbers recorded.

---

### M12 — AI runtime (local inference)

**Objective:** A local model answers prompts; the AI layer is fully userspace.

**Dependencies:** M11 (CPU/Metal backends).

**Implementation:**
- `vivanta-ai-runtime`: model package load, backend selection, tokenizer,
  context/KV management, streaming tokens.
- `vivanta-model`: full package handling (weights, tokenizer, metadata).
- CLI/test client.

**Tests:**
- End-to-end prompt→tokens on Mac (Metal) and QEMU/RK3568 (CPU).
- Model hash verified at load; tampered artifact refused.

**Exit:** Local chat on 1–3B model; benchmark recorded; **no kernel changes** in
this milestone (verified by build graph).

---

### M13 — Distributed runtime (remote workload execution)

**Objective:** A workload specified on one node executes on another.

**Dependencies:** M8 (fabric), M9 (resources), M12 (runtime).

**Implementation:**
- Fabric dispatch: `WorkloadDispatch/Result`, leases, bulk channel.
- Node runtime executes remote workload spec; results return.
- L2 (multiple independent models across nodes) + model routing.

**Tests:**
- Dispatch a workload to a remote node; result returns.
- Kill the worker mid-work; retry lands on another node (idempotent).
- Network partition: leases expire; on heal, consistent state.

**Exit:** Remote execution with idempotent retry + lease recovery demonstrated
on 3 nodes.

---

### M14 — Heterogeneous scheduler

**Objective:** Placement uses the score function and respects node admission.

**Dependencies:** M13, M9 cost model.

**Implementation:**
- `vivanta-scheduler`: score(node) with hard gates + soft weights.
- Orchestrator ↔ node offer/accept/admit loop (two-level).
- Queue-depth and thermal state feed scoring.

**Tests:**
- A model is placed on the node that best fits (memory/capability), not the
  first available.
- A node rejecting (thermal/battery) is not used.
- Scoring calibration against `vivanta-bench` numbers.

**Exit:** Placement beats naive round-robin on a defined benchmark by a
measured margin; admission honored.

---

### M15 — Distributed AI (replicas + pipeline)

**Objective:** Distributed inference beyond single-node routing: model replicas
with load balancing, then a pipeline-parallel experiment.

**Dependencies:** M13, M14, M12 (runtimes on ≥2 nodes).

**Implementation:**
- L3 replicas: multiple nodes host same model; orchestrator load-balances.
- L4 pipeline: split layers across 2 nodes (learn from llama.cpp-RPC);
  benchmark against single-node.
- KV checkpointing to CAS for resume.

**Tests:**
- Replicas: throughput scales with nodes under load; no correctness loss.
- Pipeline: distributed inference works and is benchmarked honestly
  (tokens/s, first-token, overhead).
- Kill a replica; requests migrate; service continues.

**Failure criteria:** If L4 throughput is not competitive with L1 for models
that fit one node, keep L4 as an explicit "only when it doesn't fit" mode —
which is exactly the research finding. **Do not force distribution.**

**Exit:** Replicas in production mode; L4 demonstrated and benchmarked; the
break-even table published in `docs/distributed/`.

---

### M16 — BM1684X (AIBOX-1684X as agent backend)

**Objective:** The AIBOX TPU serves a workload through the fabric as a managed
Linux node.

**Dependencies:** M11 (backend pattern validated on 2 backends), M13–M14,
`vivanta-bm1684x` FFI, physical AIBOX.

**Implementation:**
- `vivanta-bm1684x`: bmrt/bmlib FFI, BModel load/run, telemetry (bm-smi).
- Agent on AIBOX advertises TPU descriptor; model package with `artifacts[
  bm1684x]` serves BModel.
- KV/gmem handling in the AI runtime.

**Tests:**
- A BModel (e.g., Whisper / YOLO / 7B chat) runs through the fabric end-to-end.
- Telemetry (util, mem, temp) appears in ClusterView.
- Capability gating works on the agent.

**Failure criteria:** If BModel compatibility with runtime versions is broken
by vendor SDK churn, pin the SDK version and document; do not fork the SDK.

**Exit:** End-to-end TPU inference through the fabric on the physical AIBOX.

---

### M17+ — Advanced distributed AI (research track)

**Objective:** Explore L7 (disaggregated prefill/decode: compute-rich BM1684X
prefill → memory-bandwidth-rich node decode), L6 (MoE experts across nodes),
and (optionally, only on fast fabric) L5 tensor parallelism.

**Dependencies:** M15, M16, benchmarks.

**Tests:** Same rigor — benchmark every claim; no feature without a measured
win.

**Failure criteria:** Any technique that loses to single-node for models that
fit is recorded as "deferred, not broken."

**Exit:** Each technique either (a) beats local on its niche and is adopted, or
(b) is documented as out of the cluster's sweet spot with data.

---

## 3. Cross-cutting tracks

- **Benchmarks (`vivanta-bench`)**: from M11 onward every milestone publishes
  numbers.
- **Security**: capability + identity work spans M8–M16; no milestone ships
  without auth + revocation + content-verification where applicable.
- **Docs**: every milestone updates these documents + ADRs.
- **Test infrastructure**: QEMU aarch64 as CI baseline; Linux agents for
  fabric; physical nodes for M13+.

---

## 4. Dependency graph

```text
M6 ──► M7 ──► M8 ──► M9 ──► M10 ──► M11 ──► M12
                         ▲        │        │
                         │        ▼        ▼
                         └────── M13 ◄── M14
                                            │
                                            ▼
                                        M15 ──► M16 ──► M17+
```

---

## 5. Deliberate exclusions

- Tensor parallelism (L5) before M17+ and fast fabric — excluded by research.
- RDMA/CXL/DSM — excluded.
- Kernel-side AI — excluded by architecture.
- Compiler/IR stack — excluded.
- Kubernetes-style controllers — excluded; the fabric protocol replaces them.

---

*End of roadmap. Milestone gates were each written to be independently
verifiable — an agent can begin implementation at any gate whose dependencies
are satisfied.*
