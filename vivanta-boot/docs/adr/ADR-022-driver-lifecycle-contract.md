# ADR-022: Minimal Driver Lifecycle Contract

**Status:** Accepted  
**Date:** 2026-07-19  
**Related:** ADR-011 (Phase Transition), ADR-014 (Architectural Boundaries), ADR-020 (System Runtime Ownership)

---

## Context

Roadmap V3 introduces a Device Graph and Driver Component Model. Multiple device-like entities already exist in the codebase — UART, GIC, Timer — and share a common lifecycle pattern:

```
discovery → initialization → runtime
```

Gemini's architecture audit raised a concern under ADR-011: defining a `trait Driver` before two independent implementations exist could constitute premature abstraction. However, three existing subsystems (UART, GIC, Timer) already demonstrate a shared lifecycle pattern, suggesting the trait is not speculative — it documents an existing architectural boundary.

The risk is not the trait itself, but over-generalisation: adding capability methods (DMA, power management, suspend/resume) before concrete requirements exist.

## Decision

### 1. Minimal Driver trait

```rust
pub trait Driver {
    fn init(&mut self) -> Result<()>;
    fn shutdown(&mut self);
}
```

This is a **lifecycle contract**, not a complete hardware abstraction. It answers only "when does the driver start and stop?" — not "what does the device do?"

### 2. Explicitly Excluded (Premature)

The following must NOT be added to the trait until at least two independent implementations require them:

- `fn dma()` — DMA capability varies significantly between devices
- `fn reset()` — device-specific reset semantics
- `fn suspend()` / `fn resume()` — power management, deferred to post-R7
- `fn recover()` — error recovery is driver-specific

### 3. Device Lifecycle State Machine

The trait's implied state machine is documented (not enforced in the type system):

```
Discovered
    │
    ▼
Probed (matches hardware node)
    │
    ▼
Initialized (trait fn init)
    │
    ▼
Running
    │
    ▼
Shutdown (trait fn shutdown)
```

Transitions between states are the responsibility of `DriverManager`, not the driver itself.

### 4. Driver Runtime State Ownership

Drivers own their private state. `DriverManager` owns driver instances. `DeviceGraph` owns topology only — no driver state.

```rust
struct DriverManager {
    drivers: Vec<Box<dyn Driver>>,
}
```

### 5. HardwareCapabilityDescriptor (Metadata, not Capability)

Hardware capabilities are described by `DeviceDescriptor` — a data structure, not a trait. This name deliberately avoids the term "Capability", which is reserved for the security capability model planned for future milestones.

```rust
pub struct DeviceDescriptor {
    pub mmio_regions: Vec<MmioRegion>,
    pub interrupt_lines: Vec<InterruptLine>,
    pub dma_regions: Vec<DmaRegion>,
}
```

This is metadata describing what the hardware provides, not an abstraction of hardware behaviour. Metadata does not fall under ADR-011's "no abstraction before second implementation" rule.

### 6. Rationale re ADR-011

This trait does not violate ADR-011 because:

1. **Three existing entities** (UART, GIC, Timer) already share the init/shutdown lifecycle
2. The trait is a **lifecycle contract**, not a hardware abstraction — it defines when, not what
3. Capability methods are explicitly excluded until two implementations require them
4. `DeviceDescriptor` is metadata, not abstraction

## Consequences

**Positive:**

- Establishes a consistent lifecycle boundary before more drivers arrive
- Prevents ad-hoc initialisation patterns across different device types
- `DeviceDescriptor` metadata provides a common language for hardware topology without abstraction
- Driver-private state remains isolated

**Negative:**

- Drivers must box themselves (`Box<dyn Driver>`) — acceptable cost for trait flexibility
- Future capability methods that ARE shared across drivers will require trait extension — this is expected and follows ADR-011's "second implementation" trigger

**Alternatives rejected:**

- No trait, each driver has its own init function — rejected: prevents `DriverManager` from working generically
- Full capability trait with all possible device operations — rejected: premature per ADR-011
- No trait, Device Graph drives everything — rejected: drivers need a lifecycle contract
