# ADR-023: IdentityState Model

**Status:** Accepted  
**Date:** 2026-07-19  
**Related:** RFC-001 (Identity Model), RFC-004 (Recovery Seed Format), ADR-011 (Phase Transition)

---

## Context

Roadmap V1 introduces `SystemIdentity` as a kernel object. RFC-001 defines identity as a persistent, continuity-preserving concept tied to an Ed25519 keypair derived from a BIP-39 recovery seed. However, the first implementation (V1.2a) is necessarily volatile — a fresh keypair generated each boot, with no storage to persist it.

This creates a semantic tension: the same `SystemIdentity` type represents two different concepts depending on whether storage is available:

- **Volatile identity**: generated fresh per boot, lost on reboot
- **Persistent identity**: restored from storage, survives reboots

ChatGPT's audit flagged this as a risk: "later code may accidentally depend on temporary semantics." Grok suggested a marker trait. A full type hierarchy (BootIdentity → RuntimeIdentity → PersistentIdentity) was considered but rejected as premature abstraction per ADR-011.

## Decision

### 1. Identity as State, not Type Hierarchy

Identity is modelled as the current state of the system's continuity, not as a class hierarchy of identity types:

```rust
pub enum IdentityState {
    Volatile(RuntimeIdentity),
    Persistent(PersistentIdentity),
}

pub struct RuntimeIdentity {
    pub uuid: Uuid,
    pub keypair: Ed25519Keypair,
    pub genesis_timestamp: u64,
}

pub struct PersistentIdentity {
    pub uuid: Uuid,
    pub keypair: Ed25519Keypair,
    pub genesis_timestamp: u64,
    pub state_chain: StateChain,
    pub environment_chain: EnvironmentChain,
}
```

### 2. Monotonic Transition

Identity transitions are monotonic — from less continuity toward more continuity:

```
Volatile
    │
    │ (recovery seed verified, storage available)
    ▼
Persistent
```

The reverse transition (Persistent → Volatile) is not permitted unless an explicit "identity reset" mechanism is defined and documented in a future ADR. This preserves Vivanta's principle: identity continuity increases over time, never decreases.

### 3. Construction

```rust
impl IdentityState {
    /// Fresh boot — no storage available
    pub fn new_volatile() -> Self {
        Self::Volatile(RuntimeIdentity::generate())
    }

    /// Boot with verified recovery seed and storage
    pub fn from_storage(seed: &RecoverySeed, storage: &Storage) -> Result<Self> {
        let keypair = seed.derive_keypair()?;
        let chain = storage.load_state_chain(&keypair.public())?;
        Ok(Self::Persistent(PersistentIdentity {
            keypair,
            state_chain: chain,
            environment_chain: storage.load_environment_chain(&keypair.public())?,
        }))
    }
}
```

### 4. Rationale

- The enum reflects **actual system state**, not speculative abstraction
- Type hierarchy is rejected because there is only one implementation per variant today — two variants is not an abstraction, it's a union of two valid states
- Monotonic transition aligns with Vivanta's philosophy: identity is a property of continuity, not a keypair
- The enum makes it impossible to accidentally use volatile identity when persistent is expected, and vice versa

## Consequences

**Positive:**

- Eliminates semantic ambiguity: volatile and persistent identity are explicitly distinguished
- No accidental dependency on temporary semantics
- Enum match forces callers to handle both states
- Monotonic transition ensures continuity direction
- No violation of ADR-011: enum with two variants is not an abstraction, it's a sum type

**Negative:**

- Code that works with identity must handle both variants (match or if-let) — this is correct, not a drawback
- Persistent variant fields are duplicated from RuntimeIdentity (uuid, keypair) — this enables clean transition without mutation

**Alternatives rejected:**

- Single `SystemIdentity` type with a `persistent: bool` flag — rejected: makes it possible to call persistence methods on volatile data
- Full type hierarchy (BootIdentity → RuntimeIdentity → PersistentIdentity) — rejected: premature per ADR-011, only two concrete states exist
- Marker trait + generic — rejected: over-engineered for two variants
