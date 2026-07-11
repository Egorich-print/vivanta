# RFC-001: Theseus Identity Model

**Status:** Draft for discussion
**Area:** Core Architecture
**Requires:** None (foundational)
**Depends on RFCs:** None
**Supersedes:** Nothing
**Authors:** Theseus Architecture Team

---

## 1. Problem Statement

Every operating system has a notion of "this system" — typically expressed as a hostname, a machine UUID, or a hardware serial number. In conventional OS, identity is:

- **Static**: Generated once, tied to a specific motherboard or install.
- **Brittle**: Hardware replacement invalidates the identity.
- **Shallow**: Easily forged, rarely verified.
- **Single-dimensional**: One name, one machine.

Theseus OS requires a fundamentally different identity model because the system is defined by its ability to evolve hardware while preserving continuity. The Ship of Theseus metaphor demands that we can answer the question:

> "Is this the same system it was before the hardware change?"

Not as a philosophical exercise — as a technical assertion that can be cryptographically verified.

---

## 2. The Identity Primitive

The **Theseus Identity** is the system's claim of continuity. It is:

**Persistent across hardware replacement.**
**Cryptographically verifiable.**
**Independent of any physical component.**

An identity is not a name — it is a proof.

---

## 3. Desired Properties

A Theseus Identity must provide:

| Property | Description | Required for |
|----------|-------------|--------------|
| **Uniqueness** | No two systems should generate the same identity. | Core identity |
| **Continuity** | A system can prove it is the same entity across hardware changes. | Ship of Theseus |
| **Verifiability** | An observer can cryptographically verify the identity claim. | Trust |
| **Fork detection** | If a system is cloned, both copies know they share an ancestor. | Fork handling |
| **Migration** | Identity can be transferred from one physical system to another. | Ownership |
| **Ancestry** | The system knows its complete hardware evolution history. | Audit |
| **Graceful loss** | If identity is lost, the system can become a new identity without data loss. | Resilience |

---

## 4. Conceptual Model

### 4.1 Root Identity

The **Root Identity** is a cryptographic keypair generated at the moment of first system initialization.

```
System Identity = (PublicKey, PrivateKey)
```

The **Public Key** is the system's canonical identifier. It is what all other identities reference. It is embedded in:
- Every boot path.
- Every component registry entry.
- Every signed system state document.

This is analogous to a system's "DNA" — a fixed sequence that defines the original self.

### 4.2 State Chain

The system evolves through a series of **State Documents**. Each State Document captures the complete system hardware inventory and component configuration at a point in time.

```
State[N] = {
    hardware_inventory: [Component...],
    system_public_key: PublicKey,
    parent_state_hash: Hash(State[N-1]),
    timestamp: T,
    signature: Sign(PrivateKey, hash_of_this_state)
}
```

The chain is:

```
State[0] ← State[1] ← State[2] ← ... ← State[N]
```

The **Genesis State** (State[0]) is the hardware inventory at the moment of first boot, signed by the Root Key.

Each subsequent State Document proves that the system observed itself at that configuration.

### 4.3 Proof of Continuity

To prove "I am the same system that existed at State[K]":

1. Present the chain of State Documents from Genesis to the current State[N].
2. Each State Document is signed by the Root Private Key.
3. Each State Document references the hash of the previous state.

If the chain is unbroken and all signatures verify, the system has maintained identity continuity.

This is analogous to a blockchain, but:
- It is not distributed (only the system itself maintains the chain).
- There is no consensus mechanism (the Root Key is the sole authority).
- The chain length is bounded by the system's lifetime.

### 4.4 Component Identity

Each hardware component also has an identity:

```
Component Identity = Hash(Component Type + Vendor + Model + Serial Number)
```

The System State Document records the Component Identity of each installed component. This allows the system to detect:

- **Replacement**: Old component gone, new component present. System identity continues.
- **Reconfiguration**: Same components, different arrangement. System identity continues.
- **Partial change**: Some components replaced, some unchanged. System identity continues.

The hardware components are the "planks" of the Ship of Theseus. The system identity is the ship that persists through their replacement.

---

## 5. Fork Handling

A **fork** occurs when two systems share the same Root Keypair but have diverging State Chains. This can happen if:

- A system image is cloned.
- The identity keypair is copied to another device.
- A system backup is restored onto different hardware.

### Detection

Fork detection is straightforward:

```
System A: State[0] → State[1] → State[2] → State[A3]
System B: State[0] → State[1] → State[B2] → State[B3]
```

Both share Genesis (State[0]) and State[1], but diverge at State[2] versus State[B2]. Both systems can detect the fork by observing the differing state hashes.

### Response

The recommended response on fork detection is **identity divorce**:

1. The system notes that a fork exists.
2. The original system retains the Root Keypair.
3. The forked system generates a new Root Keypair.
4. The forked system signs a **Divorce Statement**:
   ```
   Divorce = {
       prior_identity: OldPublicKey,
       new_identity: NewPublicKey,
       timestamp: T,
       signature: Sign(OldPrivateKey, divorce_document)
   }
   ```

This creates a formal chain from the old identity to the new one, preserving ancestry while establishing independence.

---

## 6. Ownership and Control

The Root Keypair represents the system's identity, but who controls it?

### Model A: Hardware-Bound Identity

The Root Private Key is stored in a hardware security module (TPM, secure element) and cannot be extracted.

- **Pros**: Identity is physically bound to the device. Theft of storage does not steal identity.
- **Cons**: Migration is impossible without the original hardware.

### Model B: Extractable Identity

The Root Private Key is stored in encrypted form on the system's storage, decryptable with a user passphrase or hardware token.

- **Pros**: Identity can be migrated to new hardware.
- **Cons**: Identity can be stolen along with the storage and passphrase.

### Model C: Hybrid

The Root Private Key is split: one shard in hardware, one shard encrypted with user key. Identity exists only when both are present.

- **Pros**: Binds identity to both hardware and user. Neither alone is sufficient.
- **Cons**: Two points of failure.

### Ownership Transfer

Ownership is distinct from identity. A system retains its identity regardless of who controls it. However, ownership transfer requires the current owner to sign a transfer statement:

```
OwnershipTransfer = {
    system_identity: PublicKey,
    new_owner: OwnerPublicKey,
    prior_owner: OwnerPublicKey,
    timestamp: T,
    signature: Sign(OwnerPrivateKey, transfer_document)
}
```

---

## 7. Migration

Migration is the controlled transfer of identity from one hardware configuration to another.

### Scenario: Planned Storage Replacement

1. User replaces the storage device.
2. The new storage is installed, but the identity keypair was on the old storage.
3. The system cannot prove continuity because the identity is lost.

### Solution: Identity Store

The identity keypair must live on a **persistent identity store** that survives most hardware replacements. Options:

- **Dedicated EEPROM/secure element**: Identity survives storage, GPU, and CPU replacement.
- **Signed identity token**: User carries the identity on a USB token. Plugging it in reclaims the system.

### Migration Protocol

A migration is recorded as a special State Document:

```
Migration[N] = {
    prior_hardware: [Components...],
    new_hardware: [Components...],
    migration_type: "planned|unplanned",
    system_public_key: PublicKey,
    signature: Sign(PrivateKey, migration_document)
}
```

---

## 8. What Identity Is NOT

To prevent scope creep, identity is explicitly **not**:

| Not this | Rationale |
|----------|-----------|
| A user account | Users have identities. The system has an identity. These are distinct. |
| A network identity | The system may have multiple network addresses or none. Identity is orthogonal. |
| A trust model | Identity enables trust verification, but does not define trust policy. |
| A backup mechanism | Identity preserves continuity, not data. |
| A DRM mechanism | Identity can be used for licensing, but that is a separate domain. |

---

## 9. Open Questions

These questions require further discussion and should be resolved before RFC-001 is finalized:

| # | Question | Implications |
|---|----------|-------------|
| Q1 | What generates the Root Keypair? First boot daemon? Hardware RNG in bootloader? | Trust in the genesis moment. |
| Q2 | Where is the Root Private Key stored? (TPM, EEPROM, encrypted partition, user token) | Migration convenience vs. security. |
| Q3 | What happens if identity is permanently lost? | Recovery path must exist. |
| Q4 | How long is the State Chain retained? Forever? Rotated? | Storage cost vs. audit completeness. |
| Q5 | Can a system have multiple simultaneous identities? (e.g., dual-boot scenarios) | Identity model might need hierarchy. |
| Q6 | How does identity interact with network-defined identity? (hostname, domain, etc.) | Architectural boundary between OS identity and network identity. |
| Q7 | What is the identity lifecycle on first boot? (Generation ceremony, entropy source, backup seed) | The genesis moment is the most critical trust event. |

---

## 10. Relationship to Ship of Theseus

The identity model is the **technical mechanism** that makes the Ship of Theseus verifiable.

| Ship of Theseus concept | Technical equivalent |
|-------------------------|---------------------|
| The ship | System Identity (Root Keypair) |
| The planks | Hardware Component Identities |
| Replacing a plank | State Document showing component change |
| "Is it the same ship?" | Cryptographic verification of the State Chain |
| The ship's log | The State Chain |
| Making a copy of the ship | Fork + optional Divorce Statement |

---

## 11. Decision: Conceptual Model Accepted

After review, the following decisions are proposed for acceptance:

1. **System identity is cryptographic.** A keypair (not a UUID, not a hostname) forms the root of identity.
2. **State Chain captures hardware evolution.** Each hardware change is recorded in a signed State Document.
3. **Fork detection is intrinsic.** Divergent state chains are detectable without external coordination.
4. **Ownership and identity are separate.** Identity is the system's claim of continuity. Ownership is the user's relationship to the system.
5. **Open questions (Q1-Q7) must be resolved** before implementation begins.

---

## 12. Next Steps

If RFC-001 is accepted:

1. Resolve open questions Q1-Q7.
2. Write RFC-002: Bootstrap Architecture (which will embed identity generation in the boot sequence).
3. The identity model becomes the architectural foundation for all subsequent components.

---

*End of RFC-001*
