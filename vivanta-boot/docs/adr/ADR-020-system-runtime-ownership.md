# ADR-020: System Runtime Ownership

**Status:** Accepted  
**Date:** 2026-07-19  
**Related:** ADR-014 (Architectural Boundaries), ADR-015 (Arch Boundary Contracts)

---

## Context

Roadmap V1.1 introduces `SystemState` as the boot-time construct that aggregates all runtime state after `BootInfo` hand-off. The current design correctly states:

> `SystemState` owns runtime-coordinated state. `BootInfo` is discarded after construction.

However, without explicit ownership boundaries, `SystemState` risks becoming a God Object — a single struct accumulating 50+ fields as the system evolves. Every new subsystem would naturally be added as a field:

```rust
struct SystemState {
    hardware: HardwareState,
    memory: MemoryManager,
    scheduler: Scheduler,
    uart_driver: UartDriver,
    nand_driver: NandDriver,
    identity: IdentityState,
    service_registry: ServiceRegistry,
    // ... every future subsystem
}
```

This would violate ADR-014's dependency direction and create hidden coupling between unrelated subsystems.

## Decision

### 1. Positive Ownership

`SystemState` owns runtime-coordinated state only. This means state that:

- Is shared across multiple subsystems (coordination state)
- Defines the current execution phase (boot mode, identity resolution)
- Provides access to managers but does not contain their internal state

```
SystemState
├── HardwareState
│    └── DeviceGraph (immutable device topology from BootInfo)
├── ResourceState
│    └── MemoryManager (policy interface, not PMM internals)
├── ExecutionState
│    └── Scheduler (coordination, not thread state)
├── IdentityState
│    └── IdentityMode (Volatile | Persistent)
└── ServiceRegistry
     └── Registered services (references, not instances)
```

### 2. Negative Ownership (Prevents God Object)

`SystemState` MUST NOT own:

- **Raw hardware resources** — MMIO mappings, physical addresses, interrupt lines
- **Driver instances** — individual driver state belongs to `DriverManager`, not `SystemState`
- **Driver-private state** — DMA buffers, device-specific configuration
- **Physical memory ownership** — physical pages belong to PMM, `SystemState` references them via `MemoryManager`
- **Service implementation details** — services register themselves, `SystemState` provides access

### 3. HardwareState Immutability

`HardwareState` is immutable after construction. It is built once from `BootInfo` and never mutated. If a subsystem needs a modified view of hardware topology, it requests a new view via:

```rust
SystemState::reconcile_with_device_graph(&self, &DeviceGraph) -> VerifiedHardwareView
```

This produces a new verified view rather than mutating the canonical `HardwareState`.

### 4. DriverManager Ownership

Driver instances are owned by `DriverManager`, not `SystemState`:

```rust
struct DriverManager {
    drivers: Vec<Box<dyn Driver>>,
}
```

`SystemState` references `DriverManager` for coordination (e.g., shutdown ordering) but does not own individual drivers. This prevents the natural but wrong pattern of adding every driver as a `SystemState` field.

### 5. ServiceRegistry Ownership

Services register with `ServiceRegistry`. `SystemState` holds the registry as a reference — services manage their own lifecycle:

```rust
struct ServiceRegistry {
    services: Vec<Box<dyn Service>>,
}
```

## Consequences

**Positive:**

- Prevents God Object pattern as the system grows
- Clear insertion point: new subsystems add to `SystemState` only if they coordinate across other subsystems
- `DriverManager` and `ServiceRegistry` evolve independently
- `HardwareState` immutability preserves the `BootInfo` contract principle

**Negative:**

- Requires discipline: developers must distinguish "coordination state" from "implementation state"
- `SystemState` reference must be passed or injected rather than accessed globally — this is by design, not a drawback

**Alternatives rejected:**

- Fully flat `SystemState` with all subsystems as fields — rejected: creates God Object
- No `SystemState`, pass individual managers — rejected: too many function parameters, no coordination point
- Global mutable singletons — rejected: violates every ADR since ADR-014
