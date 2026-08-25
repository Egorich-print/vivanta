# Vivanta — Heterogeneous Compute (Resource & Accelerator Abstraction)

> **Status:** Proposed — deliverable 3.
> **Date:** 2026-08-09.
> **Companion docs:** `VIVANTA-DISTRIBUTED-ARCHITECTURE.md` (Pillar A),
> `VIVANTA-DISTRIBUTED-AI.md`.
> Target hardware: x86-64, Mac/ARM, Raspberry Pi 3B+, RK3568, Poco F1-class
> phones, FireFly AIBOX-1684X (Sophon BM1684X), future GPU nodes.

---

## 1. Design principles for heterogeneity

1. **Common interface, never common abstraction.** A BM1684X BModel and a CUDA
   module and an SPIR-V shader are not the same thing, but they can share a
   *lifecycle*. Common abstractions over the *content* are what destroys
   hardware-specific performance.
2. **The fabric never parses an artifact.** An artifact is an opaque blob; its
   interpreter is the backend that produced/understands it. The fabric only
   transports it, hashes it, and gates access to it.
3. **Vendor specificity lives in a typed extension channel**, not in the common
   trait.
4. **All device access is capability-gated.** No node-level ambient authority.
5. **Placement is policy, not kernel action.** Memory/device placement is a
   user-space decision over descriptors.

---

## 2. The resource model

### 2.1 Descriptor (static) vs state (dynamic)

- A **ResourceDescriptor** is what a node advertises once (immutable-ish
  capability statement).
- **ResourceState** is the time-varying part: load, thermal, battery, free
  memory, queue depth.

```rust
enum ResourceKind { Cpu, Memory, Gpu, Npu, Tpu, Storage, Network, Other }

struct ResourceDescriptor {
    id: u64,
    node: NodeId,
    kind: ResourceKind,
    name: String,            // e.g. "sophon-bm1684x-tpu"
    attrs: Vec<Attr>,        // arch, precision (int8/fp16...), cache, pcie
    capacity: Amount,        // bytes | cores | tops | flops | mbps
    cost: CostModel,
}

struct ResourceState {
    used: Amount,
    load: f32,               // 0..1
    thermal: Option<Celsius>,
    battery: Option<Percent>,// for phones/SBCs
    epochs: Vec<Epoch>,       // recent measurements
}
```

The descriptor is the **kernel `MemoryObject` independent** (the kernel keeps
its own MemoryObject model; the fabric descriptor is a user-space view, mapped
through the accelerator backend crate).

### 2.2 Cost model

```rust
struct CostModel {
    latency_ns: Option<u64>,
    bandwidth_mbps: Option<u64>,
    transfer_price: f64,        // per-byte cost for bulk move (network)
    power_w: Option<f32>,
    stability: Stability,        // Static | Volatile (battery/edge)
}
```

### 2.3 Memory hierarchy as attributes, not as manual control

L1/L2/L3/SLC/HBM/accelerator SRAM/TPU local memory/NUMA/DDR are **modeled** —
each exposes `{latency_ns, bandwidth}` and nothing else. The scheduler uses
these only as **cost estimation inputs** (e.g., "keep KV-cache in the fast
tier that is under 8 MB"). No cache control surfaces.

---

## 3. The accelerator API

The brief's starting hypothesis (the closed `trait Accelerator`) is **kept as a
skeleton but made capability-aware and given an opaque-artifact channel**.

```rust
pub trait Accelerator {
    fn info(&self) -> AcceleratorInfo;

    fn allocate(&mut self, req: AllocationRequest) -> Result<Allocation>;
    fn upload(&mut self, artifact: ArtifactHandle) -> Result<Allocation>;
    fn execute(&mut self, work: OpaqueWork, cap: &Capability) -> Result<ExecHandle>;
    fn wait(&mut self, handle: ExecHandle) -> Result<()>;
    fn release(&mut self, alloc: Allocation) -> Result<()>;

    /// Optional vendor-specific extension, downcast at runtime.
    fn as_vendor(&mut self) -> Option<&mut dyn Any> { None }
}
```

Type glossary:

| Type | Meaning |
|---|---|
| `AcceleratorInfo` | capabilities: precision set (int8/fp16/fp32/bf16), memory total/free, transport (PCIe gen/width, NVLink), peak ops, quant engine support. |
| `AllocationRequest` | `{ bytes, capability, ttl, preferred_class }`. |
| `Allocation` | `{ device, bytes, backend_ref, cap_required }`. |
| `ArtifactHandle` | `{ sha256, backend, opaque bytes }`. Opaque to the fabric. |
| `OpaqueWork` | `{ artifact: Option<sha>, ops: Option<OpList>, inputs, outputs, params }` — either whole-artifact or command-level. |
| `ExecHandle` | async submit; `wait` syncs. |

Two execution modes (required for full generality):

| Mode | When | BM1684X example |
|---|---|---|
| **Artifact-level** (whole model) | vendor-native, fast path | BModel → BMRuntime |
| **Command-level** | fine control over memory/KV/KV placement | CPU graph + device data copies |

This is the direct translation of the brief's requirement: *"must be possible
to represent fundamentally different execution models ... without destroying
hardware-specific optimization."*

### 3.1 Vendor extension (the "not one abstraction fits all" escape hatch)

```rust
trait Bm1684xSpecific {
    fn load_bmodel(&mut self, path: &Path, dl_ctx: &BmContext) -> Result<()>;
    fn run_bmodel(&mut self, inputs: &[u8], outputs: &mut [u8]) -> Result<()>;
    fn device_mem(&self) -> (u64, u64);         // gmem total, free
    fn copy_to_gmem(&mut self, host: &[u8], gmem: GmemRegion) -> Result<()>;
}
// use: acc.as_vendor::<Bm1684xSpecific>().ok_or(Unsupported)?
```

The common trait stays small; the vendor channel carries everything about a
specific silicon. The backing crate (`vivanta-bm1684x`) implements both the
common trait and the vendor extension.

---

## 4. Capability gating on devices

- A device is addressed only **through its capability**.
- `execute`/`upload`/`release` take the capability; the backend verifies
  `rights ⊆ device capability` and `ttl > now`.
- **Isolation**: each allocation maps to an independent execution context where
  the vendor allows (CUDA streams, Vulkan queues, separate BMRuntime contexts).
  If the vendor cannot isolate (some NPUs), `info().isolation = false` and the
  scheduler avoids co-locating sensitive workloads there.

---

## 5. Backend roadmap

| Backend crate | Hardware | Milestone | Notes |
|---|---|---|---|
| `vivanta-accelerator-cpu` | CPU | M11 | ggml / llama.cpp / ONNX Runtime CPU. First and easiest. |
| `vivanta-accelerator-metal` | Apple GPU | M11–M12 | ggml Metal over macOS. |
| `vivanta-accelerator-vulkan` | cross-GPU | M12–M13 | SPIR-V; phones (Adreno) and AMD/Intel. |
| `vivanta-accelerator-gpu` | NVIDIA/AMD | M15+ | CUDA/ROCm via Linux-agent-hosted backends. |
| `vivanta-accelerator-bm1684x` | BM1684X | M16 | FFI to BMRuntime + bmlib. |
| `vivanta-accelerator-openvino` | Intel/NPU | later | OpenVINO delegate. |

Selection is **by capability match discovered at advertise time**, not by a
hardcoded list.

---

## 6. The BM1684X reference case

Facts (KNOWN, vendor docs):

| Property | Value |
|---|---|
| Compute | 32 TOPS INT8 / 16 TFLOPS FP16-BF16 / 2 TFLOPS FP32 |
| CPU | 8× Arm Cortex-A53 @ up to 2.3 GHz |
| Memory | 16 GB LPDDR + TPU private gmem |
| Interfaces | PCIe gen, 2× GbE, video codecs |
| Software | libsophon (bmlib, BMRuntime, driver), tpu-mlir (→ BModel), tpu-nntc legacy, sophon-sail wrappers |

The FireFly AIBOX-1684X runs vendor Linux. **It is a Managed Linux node**: its
TPU is exposed through the `vivanta-bm1684x` backend. Vivanta never boots on it.
(EXPERIMENTALLY VERIFIABLE once the unit exists.)

### 6.1 Responsibilities split

| Component | Belongs to | Contains |
|---|---|---|
| `vivanta-model` | Vivanta | model package, artifact manifest (BModel hash, params) |
| `vivanta-ai-runtime` | Vivanta | load package → pick backend → run; KV/gmem handling |
| `vivanta-accelerator` | Vivanta | trait + registry (no vendor) |
| `vivanta-bm1684x` | Vivanta | FFI bindings to bmrt/bmlib; BM-specific logic; telemetry queries |
| vendor SDK | vendor | BModel format, BMRuntime, tpu-mlir, driver, firmware |

**The vendor boundary is enforced**: no BModel parsing inside Vivanta, no raw
firmware access, TPU-MLIR stays offline on the dev machine (or the AIBOX's ARM
CPU, still outside our crates).

### 6.2 What bm-smi-like telemetry maps to

`util%, memory/MEM, temperature` → `ResourceState` → feeds admission/placement.

---

## 6. The scheduling model for resources

`vivanta-scheduler` (detail in `VIVANTA-DISTRIBUTED-AI.md` but summarized):

```text
score(node) =
    compute_fit      (hard gate: can this device run this op/model?)
  + memory_fit       (bytes free vs required incl. KV estimate)
  + model_residency  (already resident? win; else − transfer cost)
  + network_cost     (payload size / transport bw + rtt)
  + latency_p95
  + power            (if reported; phones penalized unless budgeted)
  + queue_depth      (in-flight load)
  + capability_width (quant/format support, `info().capabilities`)
```

Two-level: **global placement** (fabric head picks node+backend) +
**node admission** (thermal/battery/load/capacity veto). Intra-device queue
scheduling lives inside the backend, never the fabric.

---

## 7. What we deliberately do not do

| Thing | Why |
|---|---|
| Parse BModel/cubin/SPIR-V in Vivanta | Breaks the opaque-artifact rule; vendor artifacts stay vendor-parsed. |
| Generalize a single tensor op IR for physics | Not needed early; the command-level channel covers the limited class that needs fine control. |
| Kernel directly controls GPU/TPU DMA | Device access is via capabilities into the backend, or via the Linux agent. |
| "One driver to rule them all" | The reverse; each silicon needs its own vendor channel. |

---

## 8. Acceptance tests for the abstraction

A backend is acceptable when:

1. It can `info()` accurately (so the scheduler can gate).
2. It stops a non-capability holder.
3. It runs a tiny model end-to-end (GGUF / ONNX / BModel) and reports tokens
   per second + latency (into `vivanta-bench`).
4. It can be reset on device failure (accelerator reset handling) by re-loading
   the artifact.

---

*Next: `VIVANTA-DISTRIBUTED-AI.md`.*