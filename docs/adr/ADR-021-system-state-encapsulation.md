# ADR-021: System State Encapsulation

## Status
Accepted

## Context

The Vivanta kernel currently uses a `SystemState` struct that aggregates all runtime state including:
- `IdentityState`: Volatile or persistent identity information
- `HardwareState`: Hardware description (DTB), memory map, device registry
- `BootInfo`: Direct reference to boot information

The current implementation in `kernel/src/state/mod.rs` exposes internal fields directly, and `kernel_main()` in `kernel/src/lib.rs` continues to access `BootInfo` fields after `SystemState::from_boot_info()` has consumed it. This creates an implicit dependency on `BootInfo` throughout the kernel, violating encapsulation principles.

The problem:
1. `SystemState` fields are `pub`, allowing direct mutation from anywhere
2. `BootInfo` data escapes into kernel code beyond the initialization phase
3. No clear ownership boundaries between boot-time and runtime state
4. `MemoryManager` and `DeviceRegistry` are accessed directly from `BootInfo` in `kernel_main()`

## Decision

We will encapsulate `SystemState` by:

1. **Making all fields private**: `SystemState`, `IdentityState`, `HardwareState` fields become private
2. **Providing controlled access**: Add getter methods for read-only access to state components
3. **Eliminating BootInfo escape**: All data needed from `BootInfo` must be moved into `SystemState` during `from_boot_info()`
4. **Separating concerns**: Create clear boundaries between:
   - Boot-time state (consumed during initialization)
   - Runtime state (available after initialization)
   - Persistent state (survives across reboots)

### New Structure

```rust
// In kernel/src/state/mod.rs
pub struct SystemState {
    identity: IdentityState,
    hardware: HardwareState,
    // BootInfo is NOT stored here - it's consumed during construction
}

impl SystemState {
    pub fn from_boot_info(boot_info: &BootInfo) -> Self {
        // Consume all needed data from BootInfo here
        // BootInfo is not stored and cannot be accessed later
    }
    
    pub fn identity(&self) -> &IdentityState { ... }
    pub fn hardware(&self) -> &HardwareState { ... }
    // No direct field access
}
```

### Migration Path

1. **Phase 1 (V1.1)**: Encapsulate existing fields, add getters
2. **Phase 2 (V1.1)**: Move `MemoryManager` and `DeviceRegistry` references from `BootInfo` into `HardwareState`
3. **Phase 3 (V1.2)**: Introduce proper ownership with `Arc`/`RwLock` for shared state

## Consequences

### Positive
- Clear ownership boundaries
- Prevents accidental mutation of system state
- `BootInfo` becomes truly transient (only used during initialization)
- Easier to reason about state lifecycle
- Better foundation for V1.1 Runtime Identity

### Negative
- Requires refactoring of existing code that accesses fields directly
- Slightly more verbose access patterns (getters vs direct field access)
- Need to update `kernel_main()` and `adapter_main()` to use getters

## Alternatives Considered

1. **Keep current structure**: Rejected because it maintains the encapsulation violations
2. **Use Rust's module system for encapsulation**: Partially acceptable, but doesn't solve the BootInfo escape problem
3. **Full rewrite with new types**: Too disruptive for V1.1 timeline