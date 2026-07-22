# ADR-024: Identity Model Separation

## Status
Accepted

## Context

The current identity implementation in `kernel/src/state/identity.rs` uses a single `RuntimeIdentity` struct wrapped in an `IdentityState` enum:

```rust
pub enum IdentityState {
    Volatile(RuntimeIdentity),
    Persistent(RuntimeIdentity),
}

pub struct RuntimeIdentity {
    pub id: Uuid,
    pub generation: u64,
    pub hardware_hash: u64,
}
```

This design has several issues:
1. **Semantic confusion**: `RuntimeIdentity` is used for both volatile and persistent states, but the name suggests it's only for runtime
2. **No boot-time identity**: There's no representation of identity during the boot process
3. **No clear lifecycle**: The transition from boot to runtime to persistent identity is implicit
4. **Hardware coupling**: `hardware_hash` is stored in `RuntimeIdentity` but conceptually belongs to hardware state

For V1.1 Runtime Identity, we need a clearer model that separates:
- **Boot Identity**: Identity established during boot (from BootInfo)
- **Runtime Identity**: Identity active during kernel execution
- **Persistent Identity**: Identity that survives across reboots

## Decision

We will introduce three distinct types with clear responsibilities:

### Type Definitions

```rust
/// Identity established during boot process
/// Created from BootInfo and used only during initialization
#[derive(Debug, Clone, PartialEq)]
pub struct BootIdentity {
    pub boot_id: Uuid,
    pub boot_timestamp: u64,
    pub source: BootSource,
}

/// Identity active during kernel runtime
/// Contains the current operational identity
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeIdentity {
    pub id: Uuid,
    pub generation: u64,
    pub boot_id: Uuid,  // Links to BootIdentity
}

/// Identity that persists across reboots
/// Stored in persistent storage (future: disk/flash)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistentIdentity {
    pub id: Uuid,
    pub version: u64,
    pub created_at: u64,
    pub last_updated: u64,
    pub hardware_signature: Vec<u8>,
}
```

### State Machine

```rust
pub enum IdentityState {
    /// Only during boot, before RuntimeIdentity is established
    Booting(BootIdentity),
    /// Normal runtime operation
    Runtime(RuntimeIdentity),
    /// Persistent identity loaded and active
    Persistent(PersistentIdentity, RuntimeIdentity),
}
```

### Lifecycle

1. **Boot Phase**:
   - `BootIdentity` created from `BootInfo`
   - `IdentityState::Booting(boot_identity)`
   - On successful initialization: transition to Runtime

2. **Runtime Phase**:
   - `RuntimeIdentity` created with new UUID and generation
   - `IdentityState::Runtime(runtime_identity)`
   - Links to `BootIdentity` via `boot_id`

3. **Persistent Phase** (V1.1 goal):
   - Load `PersistentIdentity` from storage
   - `IdentityState::Persistent(persistent_identity, runtime_identity)`
   - Runtime identity is derived from persistent

### Integration with SystemState

```rust
pub struct SystemState {
    identity: IdentityState,
    hardware: HardwareState,
    // ...
}

impl SystemState {
    pub fn new(boot_info: &BootInfo) -> Self {
        let boot_identity = BootIdentity::from_boot_info(boot_info);
        let hardware = HardwareState::from_boot_info(boot_info);
        
        Self {
            identity: IdentityState::Booting(boot_identity),
            hardware,
        }
    }
    
    pub fn transition_to_runtime(&mut self) {
        let runtime_identity = match &self.identity {
            IdentityState::Booting(boot) => RuntimeIdentity::from_boot(boot),
            _ => panic!("Cannot transition from non-booting state"),
        };
        self.identity = IdentityState::Runtime(runtime_identity);
    }
    
    pub fn load_persistent_identity(&mut self, persistent: PersistentIdentity) {
        // Create runtime identity derived from persistent
        let runtime = self.create_runtime_from_persistent(&persistent);
        self.identity = IdentityState::Persistent(persistent, runtime);
    }
    
    pub fn identity(&self) -> &IdentityState {
        &self.identity
    }
    
    pub fn runtime_identity(&self) -> Option<&RuntimeIdentity> {
        match &self.identity {
            IdentityState::Booting(_) => None,
            IdentityState::Runtime(r) => Some(r),
            IdentityState::Persistent(_, r) => Some(r),
        }
    }
}
```

## Consequences

### Positive
- Clear separation of concerns between boot, runtime, and persistent identity
- Explicit lifecycle transitions
- Better type safety (can't use BootIdentity in runtime phase)
- Foundation for persistent identity storage
- Easier to reason about identity state at any point in execution

### Negative
- More types to manage
- Need to update all code that uses the current `IdentityState` and `RuntimeIdentity`
- Slightly more complex state machine
- Migration effort for existing code

## Migration Path

1. **V1.1 Phase 1**: Introduce new types alongside existing ones (with `#[allow(dead_code)]`)
2. **V1.1 Phase 2**: Update `SystemState` to use new `IdentityState` enum
3. **V1.1 Phase 3**: Update all consumers to use new types
4. **V1.1 Phase 4**: Remove old `RuntimeIdentity` and `IdentityState` definitions
5. **V1.2**: Implement persistent identity loading/saving

## Alternatives Considered

1. **Keep enum with data**: `IdentityState::Volatile(RuntimeIdentity) / IdentityState::Persistent(RuntimeIdentity)` - Rejected because it doesn't solve the semantic confusion
2. **Single Identity struct with flags**: Rejected because it loses type safety and clear lifecycle
3. **Trait-based approach**: Considered for future, but adds complexity for V1.1