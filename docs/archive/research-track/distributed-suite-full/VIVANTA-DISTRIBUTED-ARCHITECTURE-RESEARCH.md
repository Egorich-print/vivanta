# Vivanta — Distributed & Heterogeneous Compute Architecture Research

> **Status:** Research report — deliverable 1 of the Distributed & Heterogeneous
> Compute Architecture research brief.
> **Date:** 2026-08-09.
> **Companion docs:** `VIVANTA-DISTRIBUTED-ARCHITECTURE.md` (target
> architecture), `VIVANTA-HETEROGENEOUS-COMPUTE.md` (resource/accelerator
> abstraction), `VIVANTA-DISTRIBUTED-AI.md` (AI/LLM execution),
> `VIVANTA-ROADMAP.md` (gated milestones), ADRs in `../adr/`.

Every major claim is tagged:

| Tag | Meaning |
|-----|---------|
| `KNOWN` | Established fact (vendor docs, specs, published systems). |
| `SUPPORTED BY EXISTING CODE` | Vivanta repo already implements this (see §2). |
| `EXPERIMENTALLY VERIFIABLE` | Testable on our QEMU/ARM/physical targets at low cost. |
| `RESEARCH HYPOTHESIS` | Plausible, not yet demonstrated. |
| `UNKNOWN` | Needs measurement; no evidence yet. |
| `UNREALISTIC` | Not sensible at the current stage. |

---

## 1. Executive summary

**Vivanta should become a capability-oriented heterogeneous distributed compute
platform — implemented as a wire protocol plus a userspace service layer above
an unchanged mechanism kernel. Not a distributed kernel, not a container
orchestrator, not a unikernel, not "Linux + an AI service".**

The single most important research finding: **all the hard parts of
heterogeneous distributed compute can be built as userspace services that speak
one protocol** — *without touching the Vivanta kernel* — because the kernel
already has the three primitive mechanisms the fabric needs:

1. **Resource abstraction with properties** — `MemoryBackend` + `MemoryResourceManager`
   + placement policy (RFC-010, ADR-025, `kernel/src/memory/`) is *exactly* the
   local pattern a distributed "remote device" backend needs.
2. **Identity primitives** — Ed25519 keypair generation/verify was already
   validated (M1-A); ADR-024 separates boot/runtime/persistent identity. Node
   identity extends this, and becomes persistent once storage exists.
3. **Capabilities** — RFC-013 is frozen but the model is preserved; distributed
   authorization can be built on a revivable capability token model.

The answer to the two final research questions (§40 of the brief):

> **Q1: Can Vivanta evolve into a capability-oriented heterogeneous distributed
> compute platform where a cluster of fundamentally different machines and
> accelerators can be treated as one programmable compute fabric, while still
> allowing hardware-specific runtimes to exploit each accelerator fully?**

**Yes.** The mechanism is a **two-channel accelerator abstraction**: a small
common capability channel (allocate / upload / execute / wait / release) plus an
**opaque artifact channel** where each vendor's native representation (BModel,
CUDA module, SPIR-V, OpenVINO blob) flows unchanged. Commonality lives in the
*protocol and the lifecycle*; specialization lives in *artifacts*, never in the
common interface. This is the same mechanism/policy and mechanism/mechanism
splits Vivanta already practices.

> **Q2: Can that architecture eventually support local and distributed
> LLM/VLM inference without turning the kernel into an AI framework?**

**Yes, and this is the design constraint that makes it clean.** Inference lives
in userspace (`vivanta-ai-runtime` + `vivanta-model` + accelerator backend
crates). The kernel provides only generic mechanisms (threads, memory objects,
IPC, device capability handles). AI is a *consumer* of the fabric, like any
other workload. Local inference is the first milestone and is deliberately
simpler than distributed inference.

---

## 2. Current Vivanta state (audit, 2026-08-09)

Grounded in `STATUS.md`, `ROADMAP.md`, `docs/architecture/master-roadmap.md`,
RFCs 001–010, RFC-013 (frozen), ADRs 011–030, and the source tree.

### 2.1 What exists (SUPPORTED BY EXISTING CODE)

| Layer | State | Relevance to distributed compute |
|---|---|---|
| PMM (`kernel/src/pmm.rs`) | ✅ PmmBitmap, reserve/alloc/free, stats | Foundation; unchanged. |
| MRM (`kernel/src/memory/`) | ✅ MemoryResourceManager, MemoryBackend, MemoryObject, placement policy (Fastest/Largest/Persistent/Balanced) | **The local resource model we lift to the cluster.** |
| VMM / AddressSpace | ✅ map/unmap/query, multi-AS isolation | Per-node isolation mechanism. |
| Scheduler | ✅ priority preemptive, dynamic RunQueue, sleep/wake | Local; never used for cluster scheduling. |
| Process model | ✅ Task/Thread/ProcessTable, EL0 entry, 5 syscalls | Base for the node service process. |
| Identity | ✅ Boot/Runtime identity; Persistent designed (ADR-024), blocked on storage | **Node identity extends this; persistent node keys blocked on storage (M7).** |
| Capabilities | ⏸ RFC-013 frozen, `check()` returns true | Distributed auth is designed as an evolution of the model, not a dependency. |
| Device graph | 📄 ADR-022 (DeviceDescriptor, DeviceGraph, Driver contract) | Basis for advertising accelerators. |
| Networking | ❌ none (documented "out of scope" until later) | **M6/M8 is the real start of the fabric.** |

### 2.2 What is missing that distributed compute requires

1. **Networking** (kernel or userspace socket layer) — nothing ships bytes yet.
2. **Userspace IPC/channels** — the node service needs a first-class channel to
   talk to the fabric.
3. **Persistent identity storage** — blocked on the storage driver; node keys
   must survive reboot.
4. **Storage** — needed for model artifacts and CAS (§12).
5. **Capability enforcement** — RFC-013 frozen; needed for the security model
   (§16), can start as user-space tokens.

### 2.3 Critical constraint the research must respect

Vivanta's own locked principles (from the audit): *mechanism before policy*,
*services over kernel features*, *no abstraction before second implementation*
(ADR-011), *device graph knows no driver state*, *BootInfo immutable*.
The distributed architecture must honor all of these — which the proposed
"userspace fabric + protocol" design does by construction.

---

## 3. What Vivanta is actually becoming (§3.1 of the brief)

Comparison of candidate models:

| Model | Verdict | Reason |
|---|---|---|
| OS (single-node) | Retained as the *mechanism layer* | It is what exists and what stays. |
| Distributed OS / multi-kernel (Barrelfish, Amoeba, Helios) | **Rejected as primary** | Kernel-level distribution collapses mechanism/policy split, makes the kernel the failure domain, and blocks portability. Vivanta's own audit already noted Barrelfish's "distributed multi-kernel where message passing is mandatory even for local SoC allocation" as a do-not-copy. |
| Cluster OS (K8s/K3s/Nomad) | Rejected as core, useful as target | Container+YAML+controllers model is the wrong unit for heterogeneous accelerators. But managed Linux nodes can speak *our* protocol without needing our kernel. |
| Distributed runtime (Ray/Dask) | Borrow ideas, reject form | Placement groups, DAG scheduling, object store ideas are good. Python-first object model is wrong. |
| Unikernel | Rejected | We keep processes/syscalls/AS; unikernels remove them. |
| Capability-based distributed system (seL4 lineage) | **Adopted as the security spine** | RFC-013's model (CNode/CSlot/derivation/revocation) is the correct authorization backbone. |
| Plan 9 / 9P namespace | Adopted as an analogy | "Cluster = one namespace over multiple nodes, each node exposing its resources" — but via a capability protocol, not a file protocol. |
| **Fabric protocol + userspace services** | **ADOPTED** | Smallest architecture that reaches the goal; kernel unchanged; hardware-specific runtimes preserved; matches all Vivanta principles. |

**Conclusion (RESEARCH HYPOTHESIS, strongly supported):** Vivanta becomes
*a single-node capability kernel plus a userspace Fabric Service Layer* that
unifies clusters of native nodes, managed Linux nodes, and external
accelerators behind one identity-aware wire protocol.

---

## 4. Comparative research: what to borrow, avoid, and why

### 4.1 Kubernetes / K3s / Nomad / Mesos
- **Borrow:** admission control; desired-state reconciliation loop; health/liveness
  checks; Mesos's *resource offers* (a node advertises capacity, a scheduler
  takes what it needs). Two-level scheduling (offer → accept) fits heterogeneous
  nodes where the node itself knows best what it can run.
- **Avoid:** containers as the unit of work; controllers/operators for every
  feature; declarative YAML as the core API; etcd-backed control plane for a
  home cluster.
- **Fit for small heterogeneous edge:** Nomad yes, K8s marginal, Mesos too heavy.

### 4.2 Ray / Dask
- **Borrow:** placement groups (affinity/anti-affinity of work); dynamic
  task graph; object store with distributed references; Dask's "graph → tasks"
  model.
- **Avoid:** Python object serialization as the wire format; centralized
  "head node does everything" as the only mode; loss of control over placement.
- **Fit:** high — Ray's resource abstraction (CPU/GPU counts per worker) is a
  proven minimum viable version of what we want, but our version is
  capability-gated and protocol-first.

### 4.3 Slurm / OpenMPI / NCCL / Gloo
- **Borrow:** Slurm partitions (node classes) and the idea of explicit resource
  reservation; MPI's communicator/group concept for collective work.
- **Avoid:** Slurm's batch-only model; MPI process model as the unit; NCCL as a
  kernel dependency.
- **Fit:** low-medium for edge; these systems assume homogeneous high-BW nodes.

### 4.4 vLLM / SGLang / llama.cpp
- **Borrow:** PagedAttention KV-cache allocation (vLLM) — critical for fitting
  large contexts in small accelerators; continuous batching (vLLM); SGLang's
  structured generation/radix KV-cache reuse; llama.cpp's **layer-sharding via
  RPC** (the proof that heterogeneous tensor-slicing across TCP works in
  practice) and GGUF as a portable quantized weight format.
- **Avoid:** rewriting vLLM; requiring their exact deployment; assuming one node
  fits one model.
- **Fit:** **highest** — llama.cpp RPC is the concrete existence proof for
  "distribute when you can't fit".

### 4.5 ONNX Runtime / OpenXLA / MLIR / TVM / OpenVINO
- **Borrow:** ONNX as an *interchange* graph format; StableHLO as a candidate
  *fabric graph IR*; OpenVINO's "delegate" pattern (a graph IR lowered to
  device-specific blobs); TVM's lowering pipeline concept.
- **Avoid:** committing to any single compiler ecosystem as the core; kernel
  work in compilers (Vivanta does not write MLIR dialects in the kernel).
- **Fit:** these are *upstream toolchains*, not the fabric. We consume their
  artifacts.

### 4.6 Vulkan compute / WebGPU
- **Borrow:** SPIR-V as a portable compute artifact; device enumeration +
  queue submission model as a *shape* for the accelerator API; WebGPU's
  adapter/device/queue split.
- **Avoid:** forcing everything through a graphics API.
- **Fit:** Vulkan is the pragmatic cross-GPU backend for CPU/GPU nodes that
  lack CUDA (phones, ARM, Intel/AMD). Reuse via `ash`/`wgpu`-style crates in
  userspace.

### 4.7 SPDK / io_uring / AF_XDP / RDMA / CXL
- **Borrow:** io_uring for efficient async I/O on Linux agent nodes; zero-copy
  ideas in the bulk-transfer channel.
- **Avoid:** RDMA/CXL/AF_XDP in the first years (UNREALISTIC for home/phone LAN).
- **Fit:** N/A until performance demands it.

### 4.8 Unikernel/distributed OS research; seL4; Redox; Genode; Plan 9; Amoeba
- **seL4 (borrow heavily):** CNode/CSlot capability tree, rights narrowing on
  derivation, cascading revocation, formal safety bounds mindset. **Avoid:**
  externalizing all memory management to user space (already noted in Vivanta
  audit).
- **Redox (borrow):** Rust-first, scheme namespace idea. **Avoid:** POSIX-first.
- **Plan 9 (borrow):** "cluster is one namespace" philosophy. **Avoid:** 9P file
  protocol as transport.
- **Amoeba/Helios (study):** object capabilities, capability naming, multi-kernel
  heterogeneity. **Avoid:** the distributed-kernel architecture.

---

## 5. Node identity, discovery, membership (§4 of the brief)

### 5.1 Identity (integrates with the Vivanta Identity Model)

```
NodeIdentity = Ed25519 public key (persistent once storage exists;
                volatile per-boot fallback)
    ├── signed resource descriptors   (what this node has)
    ├── signed membership documents   (who this node trusts)
    └── signed workload capabilities  (delegation tokens)
```

- **Ed25519 is already validated** (M1-A: keygen/sign/verify) — EXPERIMENTALLY
  VERIFIABLE as the node-key basis.
- **Device identity** = signed claim by the owning node:
  `{ node_pk, device_path (e.g. pcie:...), fingerprint }`.
- **Workload identity** = `node_pk || uuid`, embedded in every fabric message.
- **Accelerator identity** = signed device descriptor; a GPU/TPU is always
  *owned* by exactly one node and never a first-class cluster member.
- **Capability identity** = content-hash of the artifact + the capability
  (rights, owner, scope).

The four roles (cluster / node / device / workload) map onto the existing
IdentityState machine in `kernel/src/identity/` as *user-space* extensions
(ADR-031 in `../adr/`).

### 5.2 Discovery — staged by cluster size

| Phase | Mechanism | When |
|---|---|---|
| 1 | **Static membership** (`fabric.toml`: node pk + address on every node) | M8 |
| 2 | **mDNS/DNS-SD** for LAN zero-config (phones, SBCs) | M9–M10 |
| 3 | **Gossip** (signed membership diffusion) | later (M17+), only if >~40 nodes |
| 4 | Rendezvous server | only for WAN clusters |

Rationale: cluster scale here is home-lab (3–20 nodes). Static+signed
membership gives authenticity now; mDNS adds ergonomics; gossip is unnecessary
until scale demands it. (RESEARCH HYPOTHESIS — but low risk either way.)

### 5.3 Membership

```
Cluster = { signed membership document; epoch; membership_version }
Node    = { resources, accelerators, network view, health, cost model }
Join    = sign(proposed membership) → leader/quorum accepts → new doc signed
```

Trust domains are simply *membership subsets* — "trust me to run this model
class" is a per-relationship capability, not a global property. The fabric must
**never** make every node a universal root (§16).

---

## 6. Resource model (§5 of the brief)

**Decision: capability-oriented dynamic descriptors, not a closed struct.**
The kernel `MemoryBackend` pattern (RFC-010) is lifted to cluster scope.

```rust
enum ResourceKind { CpuCore, CpuThread, Memory, Gpu, Npu, Tpu, Storage, Network, ... }

struct ResourceDescriptor {
    id: u64,
    node: NodeId,
    kind: ResourceKind,
    attrs: Vec<Attr>,            // e.g. quant support, pcie width, cache size
    capacity: Amount,            // cores | bytes | flops | tops | mb/s
    latency_ns: Option<u64>,     // measured or vendor
    power: Option<f32>,          // watts
    topology: Vec<Link>,         // pcie/cxl/network links, cost model
}
```

Key properties (RESEARCH HYPOTHESIS):
1. **A resource is "held" by holding its capability.** You have a GPU if you
   hold `capability { resource, rights: [run, read_weights], ttl }`.
2. **Availability is advertised dynamically and sampled** (queue depth, thermal
   state, battery). Static capacity + dynamic state are separate.
3. **The resource model is not the kernel's `ComputeResource` struct.** The
   kernel keeps MemoryObject; the fabric keeps ResourceDescriptor. The bridge is
   the accelerator backend crate.
4. **Memory hierarchy (§23) is modeled as attributes + a cost model**, not as
   manual cache control: each descriptor carries latency/bandwidth classes that
   the placement scorer uses.

This is deliberately *smaller* than K8s resource API, *finer* than Ray, and
*capability-based* rather than name-based.

---

## 7. Accelerator abstraction (§6 of the brief)

Full design in `VIVANTA-HETEROGENEOUS-COMPUTE.md`. The research conclusions:

1. **A generic closed `trait Accelerator` that erases everything is wrong** — it
   destroys hardware-specific optimization (the brief's exact concern).
2. **The correct split is "lifecycle + capability channel" and "opaque artifact
   channel"**:

```rust
// Common channel (thin, stable)
trait Accelerator {
    fn info(&self) -> AcceleratorInfo;
    fn allocate(&mut self, req: AllocationRequest) -> Result<Allocation>;
    fn upload(&mut self, artifact: ArtifactHandle) -> Result<()>;
    fn execute(&mut self, work: OpaqueWork) -> Result<ExecHandle>;  // bytes-in, bytes-out
    fn wait(&mut self, h: ExecHandle) -> Result<()>;
    fn release(&mut self, alloc: Allocation) -> Result<()>;
}
// Vendor channel (thick, typed, optional)
trait Bm1684xSpecific { fn run_bmodel(&mut self, ...); }  // downcast via Any
```

3. **Two execution modes** must be expressible:
   - *Command-level*: primitive ops (for CPU/Metal/NPU, quant placement, KV
     management) — fine-grained control.
   - *Artifact-level*: submit the whole model (BModel → BMRuntime, TensorRT
     engine, CUDA module, SPIR-V) — vendor-native, preserves optimization.
4. **Artifacts are opaque blobs** the fabric never parses. The fabric only
   transports them and gates access by capability.
5. **Capability-based accelerator interface** (ADR-034): device access is via a
   derived capability, revocable, rights-narrowing on derivation. This directly
   implements the "accelerator isolation" requirement.

This pattern is directly informed by CUDA (module+launch), Vulkan
(device/queue/command buffer), WebGPU (adapter/device/queue), BMRuntime
(load_model/run), and llama.cpp (layer sharding).

---

## 8. Workload model (§7 of the brief)

Layers and **who owns what**:

```
Application            → Vivanta user space (session/capability holder)
  ↓
Workload               → fabric: identity + lifecycle + accounting
  ↓
ExecutionGraph (DAG)   → fabric: partitioning, placement, migration, faults
  ↓
Operators/Kernels      → backend runtime (ggml, ONNX Runtime, BMRuntime, ...)
  ↓
Device Kernels         → vendor/compiler
  ↓
Hardware Instructions  → hardware
```

- **Vivanta owns:** the Workload identity, the DAG spec, placement, migration,
  retry/checkpoint policy, accounting. All of it in *user space*.
- **Backend runtimes own:** what an "operator" is and how it executes.
- **The kernel owns none of these.** It provides threads, memory objects, IPC.

The brief's "Application → Workload → ExecutionGraph → Operators → Device
Kernels → Hardware Instructions" hierarchy is confirmed, with the ownership
boundary placed between ExecutionGraph and Operators — this is the *same
boundary* as mechanism/policy but applied at the graph level.

---

## 9. AI/ML architecture (§8 of the brief)

Detailed in `VIVANTA-DISTRIBUTED-AI.md`. Research conclusions:

### 9.1 Local inference — the first and most important milestone
llama.cpp-class engines (GGUF, Metal/CUDA/CPU) prove single-node inference of
1B–13B quantized models on consumer hardware. On our hardware:
- Mac/ARM CPU: 1–3B models comfortably, 7B slowly.
- BM1684X (32 TOPS INT8, 16 GB): 7B INT8 feasible as a whole-model BModel;
  13B possible with careful quantization/offload. (EXPERIMENTALLY VERIFIABLE
  once the box is available.)
- RK3568 NPU: ~3 TOPS INT8, 4 GB — only tiny models (≤1B) and CV pipelines.
- Poco F1-class phones: llama.cpp CPU ~2–6 tok/s on 1–3B quantized.

### 9.2 Distributed inference techniques — feasibility for a home/edge cluster

| Technique | Fits our cluster? | Why |
|---|---|---|
| Independent models per node (L2) | ✅ yes, default | Different nodes run different models; a router dispatches. |
| Replicas + load balancing (L3) | ✅ yes | Multiple nodes host same model; LB in the fabric. |
| Pipeline parallelism (L4) | ⚠️ marginal | Each layer-crossing hop pays network RTT (ms). OK for throughput across few nodes if data volume per hop is small; llama.cpp RPC proves feasibility. |
| Tensor parallelism (L5) | ❌ not now | All-reduce per token over 1 GbE/phone WiFi is loss-making. Needs fast fabric or NVLink-class links. (UNREALISTIC for the initial cluster.) |
| MoE expert distribution (L6) | 🔬 research | Attractive (router + sparse experts over nodes) but only after L4 works. |
| Disaggregated prefill/decode (L7) | 🔬 research | Interesting later: prefill is compute-bound (good on BM1684X), decode is bandwidth-bound (good on GPU/CPU). M17+ target. |

**Recommendation: Level 1 → 2 → 3 → 4, then research 6/7. Skip 5.**

The progression rationale (also §33 of the brief): each step changes one
variable at a time; local-first isolates the hard problems (model loading,
runtime, quant) from the distributed ones (placement, transport, fault
tolerance). vLLM/SGLang/Ray all confirm that serving correctness is hard before
distribution adds value. (EXPERIMENTALLY VERIFIABLE.)

### 9.3 The cluster's realistic LLM mode
The dominant pattern will be **model-grain distribution**: each model lives on
the node(s) whose memory fits it; requests route to the right node. This is
cheaper and more robust than tensor-parallel for a heterogeneous home cluster.
"Distribute a single model" is the last thing you do, not the first. (RESEARCH
HYPOTHESIS — matches llama.cpp RPC evidence: distribute *only* when one node
can't fit.)

---

## 10. Model representation (§9 of the brief)

**Decision: a Vivanta Model Package — content-addressed, multi-backend — with a
bounded graph interchange. Do NOT force one format to do everything.**

```
Vivanta Model Package (content hash = model identity, signed)
 ├── metadata     (schema, io spec, license, params)
 ├── graph        (ONNX or StableHLO; optional — null if black-box)
 ├── weights      (SafeTensors primary; GGUF accepted)
 ├── tokenizer    (tokenizers JSON / HF tokenizer)
 ├── memory       (estimate: min/int8, KV-cache requirement)
 ├── capabilities (what it can do; quant support)
 └── artifacts    (per-backend)
      ├── bm1684x → BModel
      ├── cuda    → .so/.cubin
      ├── vulkan  → SPIR-V
      ├── openvino→ compiled IR
      ├── metal   → compute pipeline / .metalir
      └── cpu     → GGUF/ONNX
```

Why this works for BM1684X: **BModel stays an opaque artifact.** Vivanta never
parses BModel; it transports it, gates it by capability, caches it by hash, and
hands it to BMRuntime through the `vivanta-bm1684x` backend. TPU-MLIR remains a
vendor/offline concern that produces BModel. (KNOWN.)

---

## 11. Placement and heterogeneous scheduling (§10–11 of the brief)

### 11.1 Scoring (used from M9)

```text
score(node) =
    compute_fit          (matches op compute class to device capability; hard gate)
  + memory_fit           (free vs required incl. KV estimate)
  + model_residency      (resident? big win; else − transfer cost)
  + network_cost         (bytes to ship / measured bw + rtt)
  + latency              (p95 measured)
  + power                (if reported)
  + queue_depth          (current load)
  + accelerator_capability (quant/formats supported)
```

This is close to the brief's suggested formula, with the addition that
`compute_fit` is a **hard gate** (a TPU that can't run a float32 transformer is
disqualified, not "scored low").

### 11.2 Scheduling model — recommendation: hybrid, hierarchical, admission-based

- **Level 1 — fabric/global scheduler** (small, centralized): picks a node and
  an accelerator for a workload class. Justified for ≤20-node clusters; simple
  and observable.
- **Level 2 — node admission**: the node (via its agent/runtime) accepts or
  pushes back (thermal, battery, current load, capability). Mesos-style offer →
  accept. This is where *hardware-specific knowledge* lives.
- **Level 3 (later) — intra-node/queue scheduling**: backend-specific (VRAM,
  KV pools, batch scheduler). Owned by the accelerator runtime.
- **Work-stealing / auction / DHT**: rejected for the first years — adds
  distributed-systems complexity without benefit at this scale.
- **Decentralized scheduling** (each node decides for itself): rejected for
  M8–M15; central+admission is the minimal correct thing.

### 11.3 Why not centralized-only, and why not fully decentralized
Centralized-only fails on heterogeneous admission (a scheduler cannot know what
a node can run); fully decentralized fails on coordination and global
optimization. The two-level split handles both. (RESEARCH HYPOTHESIS, supported
by Mesos and Nomad experience.)

---

## 12. Distributed memory and storage (§12–13 of the brief)

### 12.1 Distributed memory — deliberately conservative
- **No distributed shared memory, no remote paging, no RDMA, no CXL, no io_uring
  in the kernel** for M8–M16. (UNREALISTIC for phone/SBC/1 GbE edge.)
- What we DO build: **remote tensor buffers** — an accelerator allocation is a
  remote buffer owned by one node; data flows over the fabric's bulk-transfer
  channel (streaming upload/download), not over a memory bus.
- KV-cache memory stays **on the node that runs decode**. Prefill (compute-
  bound) can be disaggregated later; KV always stays co-located with decode.
- Memory-aware placement is part of scoring (11.1), not a separate subsystem.

### 12.2 Distributed storage — content-addressed
- `vivanta-storage` = content-addressed object store (blake3/sha256), used for
  model packages, artifacts, checkpoints, KV-cache snapshots.
- **Model artifacts → content hash → cluster cache → nearest capable node**.
  Deduplication is automatic (same model = same hash across nodes).
- Nodes cache by LRU + pinning; transfer uses the bulk channel.
- This doubles as the *checkpoint store* for fault tolerance (§15).
- Explicitly **not** a distributed filesystem, not CRDT, not a DB. Just a CAS
  with locality hints. (Matches "be conservative".)

---

## 13. Networking (§14 of the brief)

- **Transport for v1: TCP + a small framed RPC protocol** (custom or
  protobuf/capnp-flavored, length-prefixed), **QUIC later** if the edge demands
  (phones + WiFi: QUIC shines). No RDMA.
- **Cost model distinguishes**:

```text
same process (0)
same node (capability + shared memory/IPC, ~0)
LAN (1 GbE / WiFi; ~50–500 MB/s, 0.5–5 ms RTT)
WAN (asymmetric, 20–300 ms RTT)
```

- `NetworkCost(f, data_size, bandwidth, latency)` feeds the placement scorer.
- **Bulk channel** (big tensors/weights) is separate from the **control
  channel** (small RPC): control is reliable and ordered; bulk can be
  streaming/partial/retryable and is the place zero-copy/io_uring optimizations
  live later.
- No kernel networking stack needed: the node service uses a user-space socket
  layer (or a minimal POSIX-ish socket API once M6 lands). This honors
  `network-services-vision.md`.

---

## 14. Execution graph, fault tolerance (§15–16 of the brief)

### 14.1 Distributed execution graph
- A workload is a **DAG of operators with data dependencies**, partitioned by
  the fabric into per-node sub-graphs.
- Graph concerns: partitioning (at op or layer granularity), placement (11),
  migration (re-place a sub-graph), retries, checkpointing (via CAS), backpressure
  (flow control on bulk channel), cancellation (propagate cancel cap), partial
  failure (see 14.2), speculative execution (later).
- The DAG is **data** (serializable), not code in the kernel.

### 14.2 Fault tolerance — assume nodes WILL fail
Design semantics (ADR-038):

| Failure | Handling |
|---|---|
| Node disappearance | Heartbeats (timeout = grace) → drain in-flight, re-place work, revoke node's caps. |
| Accelerator reset | Backend detects device error → mark allocation dead → retry on same node (re-load artifact) then elsewhere. |
| Network partition | Partition-aware: leases for resources (with TTL), so on healing the node with the valid lease continues. |
| Process crash | Node service restarts; state restored from CAS checkpoint. |
| Model load failure | Retry once on node, once elsewhere; log; report. |
| OOM | Pre-reserve memory in placement; on failure migrate. |
| Thermal/battery | Node admission rejects new work; existing work completes or migrates. |
| Corrupted model | Content-hash verification at load → refuse; fall back to another node/copy. |

- **Idempotency:** every fabric message carries `(workload_id, op_seq)`; retries
  are deduplicated at the receiver.
- **At-least-once + idempotent execution**, never exactly-once for compute
  (impossible without checkpoint/commit).
- **Capability revocation** on membership change: a node that leaves loses its
  derived capabilities; a re-joining node re-authenticates.
- **No quorum/Paxos/Raft** for the fabric in M8–M16 — the cluster is small and
  a single control node with lease-based state is enough. Multi-control-node
  consensus is M17+ if ever.

---

## 15. Security model (§17–18 of the brief)

- **Node auth:** every fabric message is signed by the sender node's Ed25519 key;
  sessions establish symmetric keys (per-pair, rotating).
- **Workload auth / delegation:** workload gets a capability (rights, scope,
  TTL) issued by its owner node; derivation only narrows rights.
- **Least privilege:** every resource/device access is a capability; nothing
  is ambient.
- **Accelerator isolation:** device access only via capabilities to the owning
  node's backend; the backend isolates (separate contexts/streams where the
  vendor supports it).
- **Memory isolation:** fabric never shares address spaces across nodes; remote
  buffers are owned and access-controlled by the backend.
- **Model access control:** model package capability gates who may load it.
- **Encrypted transport:** v1 = Noise-protocol-style key exchange over TCP or
  TLS; QUIC+TLS when adopted. (Reticulum's per-link encryption is a good model.)
- **Signed workloads & models:** workload specs and model packages signed by
  their issuer; content-hash verifiable.
- **Remote attestation:** NOT in v1 (phone/edge lacks TPM broadly; UNKNOWN, low
  priority). Secure boot of native Vivanta nodes comes with the boot work.
- **Trust domains:** membership subsets; **a node is authoritative only over its
  own devices**, never over the cluster (the brief's "must NOT turn every node
  into a universally trusted root" is enforced structurally).
- **AI model security:** model artifacts are untrusted code-equivalent (BModel,
  CUDA, SPIR-V execute on hardware). Therefore: sign them, capability-gate their
  execution, sandbox the backend where possible, audit provenance, and treat
  "arbitrary custom kernel" as a privilege escalation vector. (ADR for model
  security.)

---

## 16. BM1684X as the reference heterogeneous backend (§19 of the brief)

Full design in `VIVANTA-HETEROGENEOUS-COMPUTE.md` §BM1684X. Research findings:

- **The AIBOX-1684X runs vendor Linux.** It will never boot Vivanta's kernel in
  the near term → it is a **Managed Linux node** (agent) and its TPU is an
  **external accelerator** accessed via that agent. This is the *transitional
  architecture* (§20 of the brief) in action.
- **Vendor boundary:** BModel (compiled artifact), BMRuntime (runtime), bmlib
  (device mgmt), TPU-MLIR (compiler) — we consume BModel/BMRuntime through an
  FFI shim; TPU-MLIR stays offline (dev-machine), producing BModel.
- **What goes where:**
  - `vivanta-bm1684x` — FFI bindings to BMRuntime/bmlib + backend implementing
    the common Accelerator trait + `Bm1684xSpecific` extensions. Vendor-isolated.
  - `vivanta-ai-runtime` — model package loading, graph dispatch, KV handling,
    engine orchestration; backend-agnostic.
  - `vivanta-accelerator` — the trait + registry + generic API. No vendor code.
- **What remains vendor-specific:** BModel format, BMRuntime, firmware,
  TPU-MLIR, driver `.ko`. (KNOWN — the goal is vendor isolation, not pretending
  the dependency doesn't exist.)
- **BM1684X telemetry** (bm-smi-like: usage, memory, temperature) surfaces as
  resource state to the fabric.

---

## 17. Distributed LLM progression (§20 of the brief)

Levels and realistic plan:

| Level | What | Status |
|---|---|---|
| 1 | One model, one node | M12 (Local AI) |
| 2 | Multiple independent models across nodes | M12–M13 |
| 3 | Model replicas + load balancing | M13–M14 |
| 4 | Pipeline-parallel inference | M15 (experimental) |
| 5 | Tensor-parallel inference | **M17+; only on fast fabric** |
| 6 | MoE expert distribution | M17+ research |
| 7 | Disaggregated prefill/decode | M17+ research |

**The realistic progression is Levels 1–4, then research 6/7.** Jumping to
tensor parallelism on 1 GbE is the classic mistake the brief warns about.

---

## 18. Heterogeneous AI example (§21 of the brief)

Concrete conceptual scenario:

```
Mac / ARM CPU  (orchestration + control plane + routing)
   │
   ├── LLM routing: which model, where
   ├── small CPU/Metal models (chat, embeddings)
   │
   ├── AIBOX BM1684X (managed Linux node + agent)
   │      ├── BModel 7B INT8 — main LLM inference (chat)
   │      ├── vision models (YOLO-class) — CV pipelines
   │      └── Whisper-class ASR
   │
   ├── GPU node (CUDA/ROCm)
   │      ├── large model hosting (13B+)
   │      └── embeddings/rerank batch
   │
   └── RK3568 + phone nodes (opportunistic, tiny)
          ├── 1B models / on-device edge tasks
          └── sensors/CV when idle, battery-aware
```

- **Which workloads go where** (decision table in `VIVANTA-DISTRIBUTED-AI.md`):
  compute-bound prefill/vision → BM1684X; memory-bound decode/large models →
  GPU node; latency-tolerant tiny tasks → phones/RK3568; orchestration → Mac.
- This demonstrates heterogeneous value concretely: the BM1684X's 32 TOPS INT8
  does the heavy *prefill and CV* work at low power; the GPU node does what only
  a GPU can do; the phones absorb bursty, latency-tolerant load. (RESEARCH
  HYPOTHESIS — to be benchmarked at M15.)

---

## 19. Smartphone/edge clusters and hardware taxonomy (§22, §31 of the brief)

- **Opportunistic nodes are real**: Poco F1-class phones (6 GB RAM, Adreno 630)
  run llama.cpp-class CPU inference; battery/thermal-aware admission makes them
  useful *when idle*, never latency-critical.
- **Node taxonomy (the single most important architectural decision of the
  transitional phase):**

| Kind | Runs | Role |
|---|---|---|
| **Native Vivanta node** | Vivanta kernel | Full capabilities; the research home. |
| **Managed Linux node** | Linux + Vivanta agent | Exposes CPU/GPU/NPU/TPU (incl. AIBOX-1684X) via fabric protocol. |
| **External accelerator** | Behind a host runtime | Accessed through the owning node's backend; never a cluster member. |

- RK3568: NPU ~3 TOPS INT8 → tiny-model/CV node; also a candidate for native
  Vivanta (it is the primary hardware bring-up target).
- Phones: Android/Linux agents, opportunistic compute + sensor fusion; battery
  status is a first-class resource attribute.

---

## 20. Transitional architecture and the Linux agent (§32 of the brief)

**The pragmatic path is validated:**

```text
             Vivanta Cluster Control Plane (fabric)
                    │
        ┌───────────┴────────────┐
   Native Vivanta          Managed Linux Agent
        │                   │
   local devices       CUDA / BM1684X / Vulkan / NPU
```

- The control plane is **the same wire protocol** on both sides — native nodes
  and agents are indistinguishable to the fabric except by node kind.
- This lets Vivanta control GPU/BM1684X/phone hardware **years before Vivanta
  itself has drivers** for them. (KNOWN from llama.cpp-RPC + libsophon: both are
  userspace-hostable.)
- The agent is a normal Linux daemon (Rust), not a kernel module. It binds to
  the fabric, advertises its node's resources, and executes workloads via
  backends (llama.cpp/ggml, ONNX Runtime, libsophon FFI, Vulkan).

**Evaluation:** adopt this immediately (M8). It is the shortest path to a
*real* heterogeneous cluster and validates the protocol before the kernel
networking work is complete.

---

## 21. CPU cache / memory hierarchy (§23 of the brief)

- **Do NOT control caches.** Model the hierarchy as attributes and cost:
  L1/L2/L3/SLC/HBM/accelerator SRAM/TPU local memory/NUMA/DDR each carry
  (latency_ns, bandwidth, capacity) in the resource descriptor.
- The **placement scorer uses these** to decide e.g. "put KV on DDR near the
  decode core" vs "prefill on TPU gmem". It is *scheduling cost estimation*, not
  manual cache management.
- For AI: the dominant cost is **memory bandwidth**, not FLOPs, for decode
  (§9.3); modeling it as bandwidth class is more valuable than modeling cache
  geometry. (KNOWN from LLM serving literature — the memory-bound decode
  bottleneck is well established.)

---

## 22. Compiler / runtime boundary (§24 of the brief)

- **Does Vivanta need its own IR? No** — not as a compiler IR. It needs a
  *bounded interchange representation* for the model package graph (ONNX or
  StableHLO as interchange, never as the execution format).
- The pipeline:

```text
Model → Vivanta Model Package (graph interchange + weights + artifacts)
     → backend selection (fabric placement)
     → vendor/compiler toolchain (offline: TPU-MLIR, nvcc, etc.)
     → executable artifact (BModel, cubin, SPIR-V, GGUF)
     → Vivanta runtime (vivanta-ai-runtime → accelerator backend)
     → hardware
```

- Building our own IR/compiler stack (MLIR/TVM/IREE) is **out of scope** —
  those are multi-year projects better consumed as artifacts. (UNREALISTIC for
  Vivanta's team size; KNOWN from the ecosystem.)

---

## 23. Rust workspace evolution (§25 of the brief)

Proposed workspace (justified in `VIVANTA-DISTRIBUTED-ARCHITECTURE.md`):

```
vivanta-core          identity + capability primitives (no_std-safe core types)
vivanta-fabric        wire protocol, messages, serialization
vivanta-cluster       membership, discovery, node view
vivanta-resource      ResourceDescriptor, cost model
vivanta-scheduler     placement scoring/policy
vivanta-runtime       node service runtime, executor, lifecycle
vivanta-accelerator   Accelerator trait + registry (no vendor code)
vivanta-model         model package, artifacts, CAS client
vivanta-storage       content-addressed store (CAS)
vivanta-ai-runtime    local/distributed inference engine orchestration
vivanta-bm1684x       FFI backend (vendor-isolated)
vivanta-agent         managed-Linux agent daemon
vivanta-orchestrator  control plane daemon (fabric head)
```

**Boundaries:** `vivanta-core ← everything`. `vivanta-accelerator` depends only
on `vivanta-core` + `vivanta-resource` (never on `vivanta-model` or vendors).
Backend crates (`vivanta-bm1684x`) depend on `vivanta-accelerator`. The kernel
crate stays as-is and is **not** a dependency of any fabric crate. This avoids
circular dependencies by construction.

---

## 24. Kernel vs userspace boundary (§26 of the brief)

**Kernel (unchanged, mechanism-only):**
- isolation, task/thread, scheduler primitives, memory protection
- IPC (message passing, shared memory — M6 work)
- device access via capabilities (future; device graph)
- network primitives (packet transport only, per `network-services-vision.md`)

**Userspace (everything distributed):**
- cluster membership, discovery, identity for nodes
- model management, AI scheduling, distributed execution
- accelerator runtimes, model loading, orchestration
- telemetry, policy

**The kernel must NOT become an AI framework.** This is guaranteed by the
boundary: nothing in `kernel/` imports `vivanta-model` or `vivanta-ai-runtime`;
the only shared crate is the primitive `vivanta-core`-style types.

---

## 25. Benchmarking (§34 of the brief)

`vivanta-bench` measures:
- startup latency; model load time; tokens/sec; first-token latency; memory
  usage; network overhead; CPU overhead; accelerator utilization; model
  transfer time; scheduling latency; migration cost; failure recovery time.

**Distributed comparison:**
```
local inference  vs  remote inference  vs  distributed inference
```
- **Break-even analysis**: compute (a) per-token data volume for a distributed
  model (activations/KV per step), (b) measured LAN bandwidth/RTT, (c) local
  per-token time. Rule of thumb from llama.cpp-RPC: **distribution pays off when
  one node cannot fit the model** (memory-bound), not when one node is "too
  slow" (compute-bound) — because every remote op pays a network round-trip.
- This benchmark suite is a **hard gate** for M15 (do not claim distributed AI
  without numbers).

---

## 26. Assumptions, unknowns, risks (§37 of the brief)

### Assumptions
- Ed25519-based node identity is sufficient (EXPERIMENTALLY VERIFIABLE — M1-A
  validated primitives).
- TCP + framing is enough for v1 transport on home LAN (EXPERIMENTALLY
  VERIFIABLE; llama.cpp-RPC works over TCP).
- Model-grain distribution beats tensor-parallel at this scale (RESEARCH
  HYPOTHESIS; benchmark at M15).
- The fabric can run entirely in userspace (RESEARCH HYPOTHESIS — no known
  blocker, but the missing kernel IPC/socket layer is prerequisite M6).

### Unknowns
- Actual BM1684X memory headroom for 7B/13B INT8 under vendor OS.
- Phone WiFi reliability for sustained inference.
- Which inference engine(s) to bind first (ggml vs ONNX Runtime) — measure at M12.
- Whether the fabric needs QUIC over TCP for phone nodes (measure at M13).
- Real multi-node scheduling behavior at M15 (score-function calibration).

### Risks
1. **Over-engineering the fabric before the kernel is usable** (mitigate: gates).
2. **Scope creep into compiler/AI-framework territory** (mitigate: strict
   boundary in §24, "no abstraction before second implementation").
3. **Distributed AI before local AI** (mitigate: M12 before M13–M15).
4. **Dependency on vendor blobs** (BM1684X) — mitigate: opaque-artifact channel,
   the box is optional.
5. **Single-developer capacity** — the roadmap is gated to one milestone at a
   time (existing Vivanta rule).
6. **Phone/edge unreliability** — admission + leases + idempotency from day one.

### Unrealistic goals (explicitly out of scope)
- RDMA/CXL/remote-paging v1.
- Distributed shared memory / transparent memory pooling.
- Tensor parallelism over 1 GbE before M17+.
- Remote attestation across phone/edge.
- Building an MLIR/TVM-class compiler stack.

---

## 27. Where kernel work is required (vs userspace)

- **Kernel work:** M6 (IPC, minimal sockets), M7 (storage driver — enables
  persistent node identity + CAS), capability enforcement revival (M10+).
- **Userspace:** everything else in the roadmap.
- **Compiler work:** none inside Vivanta; consume vendor artifacts.
- **Hardware work:** BM1684X box (for M16), GPU node (M15 benchmarks),
  RK3568/phones (M13+).
- **Distributed-systems research:** scheduling score calibration, fault
  semantics, KV-cache disaggregation (M15+).

---

## 28. Research conclusions — the smallest coherent architecture

The smallest architecture that reaches the goal, preserving everything
validated:

1. **Keep the kernel as-is** (mechanism layer, M1–M5 work untouched).
2. **Add a userspace fabric protocol + node service** (`vivanta-fabric`,
   `vivanta-cluster`, `vivanta-runtime`) — M8–M10.
3. **Add the accelerator abstraction + local AI runtime** (`vivanta-accelerator`,
   `vivanta-ai-runtime`, `vivanta-model`) — M11–M12.
4. **Add distribution on top** (scheduler, distributed runtime, agent) — M13–M16.
5. **BM1684X is a managed-Linux agent backend, never a kernel dependency.**

This is *not* Kubernetes-in-Rust; it is `capabilities + resources + workloads +
execution graphs + accelerators + distributed communication`, expressed as one
wire protocol with hardware-specific artifacts kept opaque.

---

*Next: `VIVANTA-DISTRIBUTED-ARCHITECTURE.md`.*
