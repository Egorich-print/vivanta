# ADR-021: BootInfo Escape Prevention

**Status:** Accepted  
**Date:** 2026-07-19  
**Related:** ADR-015 (Arch Boundary Contracts), RFC-008 (Boot Protocol), RFC-009 (Platform Capability Model)

---

## Context

ADR-015 establishes `kernel_main(info: &BootInfo)` as the immutable entry point signature — the kernel receives a borrowed reference to `BootInfo`. ADR-020 defines `SystemState::from_boot_info()` as the constructor that aggregates runtime state from `BootInfo` and then discards the original.

The architectural principle states:

> `BootInfo` is immutable. It exists only to transfer information from the bootloader into the kernel. It must never become global mutable runtime state.

However, the current design has no formal enforcement. Nothing prevents code from accidentally retaining a reference into `BootInfo`-owned memory:

```rust
struct HardwareState {
    fdt: &'static FdtBlob,  // reference into BootInfo memory — ILLEGAL
}
```

If `BootInfo` memory is later reclaimed or unmapped (by the Memory Resource Manager), such references become dangling pointers. Even if reclamation never occurs, retained references create implicit lifetime coupling between the bootloader and the runtime — the exact coupling Vivanta's architecture forbids.

The invariant must be:

> After SystemState construction, BootInfo lifetime must not influence runtime lifetime.

## Decision

### 1. Copy Semantics

`SystemState` construction copies all needed data from `BootInfo`. Subsystem constructors use explicit copy methods:

```rust
impl SystemState {
    pub fn from_boot_info(info: &BootInfo) -> Self {
        Self {
            hardware: HardwareState::copy_from(info),
            memory: MemoryState::copy_from(info),
            execution: ExecutionState::new(),
            identity: IdentityState::new_volatile(),
            services: ServiceRegistry::new(),
        }
    }
}

impl HardwareState {
    pub fn copy_from(info: &BootInfo) -> Self {
        Self {
            devices: info.mmio_regions.iter().map(|r| DeviceDescriptor::from(r)).collect(),
            memory_geometry: info.memory_geometry,
            // No &'static references from BootInfo may escape
        }
    }
}
```

### 2. Invariant

```
After SystemState construction:
BootInfo lifetime must not influence runtime lifetime.
```

This means:

- No `&'static` references from `BootInfo` may be stored in any kernel object
- No raw pointers into `BootInfo`-owned memory may be retained
- All kernel data structures must own their storage

### 3. Forbidden Pattern

```rust
// ILLEGAL — reference into bootloader-owned memory
struct HardwareState {
    fdt: &'static FdtBlob,
    compatible: &'static str,
}
```

**Allowed alternative:**

```rust
// CORRECT — owns its storage
struct HardwareState {
    devices: Vec<DeviceDescriptor>,
    compatible: String,
}
```

### 4. Example of the Principle in Practice

```rust
pub unsafe fn kernel_main(info: &BootInfo) -> ! {
    // Only place BootInfo is accessed
    let state = SystemState::from_boot_info(info);

    // After this point, BootInfo must not be referenced.
    // The compiler prevents further use via Rust ownership.
    // info is no longer accessible.

    vivanta_main(state)
}
```

### 5. Validation

At minimum, a compile-time check that `SystemState` fields contain no references to `BootInfo`. If the system gains testing infrastructure, a runtime assertion that `BootInfo` memory is not accessed after construction.

## Consequences

**Positive:**

- Runtime lifetime is fully decoupled from bootloader memory
- `BootInfo` memory can be reclaimed or unmapped after construction
- Prevents a common source of undefined behaviour in kernel development
- Aligns with Vivanta's philosophy: runtime is independent of its boot origin

**Negative:**

- Copy semantics introduce a one-time memory and performance cost during boot
- Prevents zero-copy access to boot-time data structures
- Some `BootInfo` fields that are truly static (e.g., architecture enum) must still be copied — minimalist overhead

**Alternatives rejected:**

- Borrow `BootInfo` for the entire kernel lifetime — rejected: creates implicit lifetime coupling, prevents reclamation
- Reference counting `BootInfo` — rejected: keeps bootloader memory alive indefinitely, more complex than copy
- Ownership transfer (consuming `BootInfo`) — rejected: `kernel_main(&BootInfo)` signature is locked by ADR-015
