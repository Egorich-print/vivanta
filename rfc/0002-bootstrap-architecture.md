# RFC-002: Bootstrap Architecture

**Status:** Draft for discussion
**Area:** Core Architecture
**Requires:** RFC-001 (Identity Model), RFC-001.5 (Identity Utility Model)
**Depends on RFCs:** RFC-001, RFC-001.5
**Supersedes:** Nothing
**Authors:** Theseus Architecture Team

---

## 0. Mandatory: Define "Continuity"

Before defining the bootstrap architecture, we must define what "the same system" means. Without this definition, the identity model has no success criteria.

### 0.1 Formal Definition

A system S at time T2 is **continuous** with system S at time T1 if and only if:

```
RootKeypair(S, T2) == RootKeypair(S, T1)
```

AND

There exists a chain of State Documents from State(T1) to State(T2) where:
- Each State Document is signed by the Root Private Key
- Each State Document references the hash of the previous State Document
- No State Document in the chain contains a contradiction (same component replaced twice without an intervening removal)

This is the **cryptographic continuity** model. It is the only formalism that matters for the architecture. Philosophical questions ("is it really the same ship?") are resolved by cryptographic verification: if the keypair matches and the chain is unbroken, the system is continuous.

### 0.2 Continuity Scenarios

| Scenario | Continuous? | Rationale |
|----------|-------------|-----------|
| Storage replaced, keypair recovered from seed | ✅ Yes | Keypair matches, chain updated with new storage component |
| CPU replaced, keypair on new CPU's secure element | ✅ Yes | Keypair transferred, chain updated |
| Full hardware replacement, keypair migrated via seed | ✅ Yes | Keypair restored, chain extended from genesis |
| Storage cloned, both copies boot | ✅ Yes (both) | Both have the same keypair and valid chains — this is a fork |
| Storage cloned, one copy restored from seed with new keypair | ❌ No (clone) | New keypair means new identity. Previous identity is lost to the clone. |
| Storage dies, no seed backup | ❌ No | Keypair lost. System is a new entity. |
| Full OS reinstall without keypair preservation | ❌ No | New keypair generated at first boot. Previous identity is gone. |
| Recovery Seed used 50 years later on different hardware | ✅ Yes | Keypair restored from seed, chain extended from genesis. Continuity spans decades. |

### 0.3 The Boundary Condition

The most important boundary:

> **A system loses continuity when the Root Private Keypair is permanently lost and no Recovery Seed exists.**

This is the atomic event of "system death" in Theseus. Everything else — storage replacement, CPU replacement, full hardware migration — preserves continuity if the keypair survives.

---

## 1. Problem

The bootstrap architecture must solve three problems:

1. **Identity Genesis**: How does a system create its identity on first boot, when there is no prior state?
2. **Identity Independence**: How does the identity survive replacement of the component it is stored on?
3. **Recovery**: How does the system prove continuity after hardware change?

The M1 constraint from R1 is:

> Replace storage on the Xiaomi Redmi Note 7, boot, and the system proves it is the same entity.

This means identity must be recoverable after storage is replaced.

---

## 2. Architectural Constraint: Identity Independence

**The Root Keypair must not depend on the component it is designed to survive replacement of.**

For M1, the replaceable component is **storage**. Therefore:

```
❌ Root Keypair stored ONLY on system partition (dies with storage)
✅ Root Keypair recoverable independently of storage (Recovery Seed)
```

This is the central architectural constraint of the bootstrap design.

---

## 3. Bootstrap Architecture

### 3.1 First Boot Ceremony (Genesis)

The first boot establishes the system's identity. No prior state exists.

```
Boot sequence:
  1. Power on
  2. Bootloader loads
  3. Hardware inventory collected
  4. No identity exists → Genesis mode
       a. Generate Root Keypair (Ed25519 or similar)
       b. Display Recovery Seed on screen (BIP-39 style)
       c. Write Recovery Seed to system partition (/boot/keypair.seed)
       d. Create Genesis State Document:
            Genesis = {
                system_public_key: RootPublicKey,
                hardware_inventory [initial components],
                timestamp: T,
                signature: Sign(RootPrivateKey, genesis_content)
            }
       e. Write Genesis to system partition (/theseus/state/genesis)
  5. Boot continues into the OS
```

The **Recovery Seed** is the critical output. It is:
- Displayed on screen during first boot
- Written to the system partition (for automated recovery)
- The user is prompted to back it up (written down, stored elsewhere)

### 3.2 Identity Storage Model

The Root Keypair exists in two forms:

| Form | Location | Purpose |
|------|----------|---------|
| **Primary Keypair** | System partition (encrypted) | Normal operation, signing |
| **Recovery Seed** | External (user-managed) + system partition | Recovery after storage replacement |

**The system partition copy of the Recovery Seed provides automatic recovery when the new storage has the data partition attached.** The external copy provides recovery when everything is lost.

### 3.3 Normal Boot (Identity Exists)

```
Boot sequence:
  1. Power on
  2. Bootloader loads
  3. Hardware inventory collected
  4. Identity exists → Normal mode
       a. Load Root Keypair from system partition
       b. Compare current hardware inventory with last State Document
       c. If unchanged:
            - Boot normally
       d. If changed:
            - Create new State Document with current inventory
            - Sign with Root Private Key
            - Append to State Chain
            - Boot normally
```

### 3.4 Storage Replacement Recovery (M1 Scenario)

This is the critical path that M1 must prove.

```
Scenario:
  - Storage device failed or replaced
  - New storage installed
  - No identity on the new storage

Boot sequence:
  1. Power on
  2. Bootloader loads from new storage
  3. Hardware inventory collected
  4. No identity found on new storage → Recovery mode
  5. Check if user data partition is present (old storage or migrated data)
  6. If user data found:
       a. Look for /identity/recovery.seed in user data
       b. If found: restore Root Keypair from seed
       c. Verify that the recovered keypair matches the Genesis State Document
       d. Create Migration State Document:
            Migration = {
                prior_storage: [old storage identity],
                new_storage: [new storage identity],
                restoration_type: "seed_recovery",
                system_public_key: RootPublicKey,
                timestamp: T,
                signature: Sign(RootPrivateKey, migration_content)
            }
       e. Append to State Chain
       f. Boot continues
  7. If user data NOT found:
       a. Prompt user for Recovery Seed
       b. User enters seed
       c. Restore Root Keypair from seed
       d. No prior state available → system is "same identity, no prior state"
       e. Create Genesis-from-recovery State Document
       f. Boot continues
```

### 3.5 The M1 Continuity Loop

The complete M1 experiment, from first boot to continuity proof:

```
Step 1: First boot (Device A, Storage A)
        → Generate Root Keypair
        → Display Recovery Seed (RECOVERY_SEED_01)
        → Genesis State Document (S0)
        → Write user data to storage

Step 2: Power off, replace Storage A with Storage B
        (Storage B is blank, no identity)

Step 3: Boot (Device A, Storage B)
        → No identity on Storage B
        → User data partition from Storage A is NOT present
        → Prompt for Recovery Seed
        → User enters RECOVERY_SEED_01
        → Root Keypair restored
        → Verify: restored keypair matches Genesis (S0) public key
        → Continuity proved: same identity
        → Migration State Document created (S1: storage replaced)

Step 4: System boots successfully
        → State Chain: S0 (genesis) → S1 (storage migration)
        → User can verify: "This is the same system."
```

---

## 4. Why This Architecture

### 4.1 M1 Validation

This architecture proves the core Theseus thesis:

> **A computing system can survive the death of its primary storage and remain itself.**

The proof is cryptographic: the recovered keypair matches the Genesis keypair, and the State Chain records the transition.

### 4.2 What It Does NOT Prove

M1 does not prove:
- Hot hardware replacement (requires reboot)
- CPU/GPU/motherboard independence (storage only)
- Fork detection (single device)
- Multi-device trust (single device)
- Autonomous agent continuity (no agents)

These are explicitly deferred.

### 4.3 Why the Recovery Seed Works for M1

| Concern | Mitigation |
|---------|------------|
| User loses the seed | Seed is written to the system partition AND displayed on screen with a prompt to back it up. For M1, the system partition copy covers the primary scenario. |
| Seed is too long | BIP-39 12-word seed (128 bits of entropy) is standard and user-friendly. |
| Seed is insecure | M1 is a proof of concept, not a production security model. Security hardens in later milestones. |
| Seed conflicts with "zero friction" philosophy | Seed generation is a one-time event during first boot. Normal operation requires no seed interaction. |

---

## 5. What Is NOT in RFC-002

| Concept | Excluded Because |
|---------|-----------------|
| Fork detection | M1 has no clone scenario. |
| Ownership transfer | Single user, no transfer. |
| Multi-device identity | Single device. |
| Hotplug hardware detection | Storage replacement is cold (power off, replace, power on). |
| Secure element / TPM integration | Adds complexity without benefit for M1. Recovery Seed is sufficient. |
| Network-based recovery | Adds network dependency. M1 must work offline. |
| Key rotation | Premature optimization. |

---

## 6. Open Questions

| # | Question | Implications |
|---|----------|-------------|
| Q1 | Should the Recovery Seed be written to the data partition or the system partition? | Data partition survives storage replacement if the user migrates it. System partition does not. |
| Q2 | What happens if the user replaces both storage AND motherboard simultaneously? | The system loses both the identity (storage) and any hardware-based verification. Recovery Seed is the only path. |
| Q3 | How does the bootloader participate in identity recovery? | The bootloader must be able to detect "no identity" and enter recovery mode. This constrains bootloader design. |
| Q4 | Is the State Chain stored on the system partition or a dedicated partition? | If on the system partition, it dies with storage. A dedicated identity partition could survive. |
| Q5 | What format does the Recovery Seed use? | BIP-39 is the standard for human-friendly seed encoding. Alternatives: raw hex, QR code. |

---

## 7. Relationship to Previous RFCs

| RFC | Contribution to RFC-002 |
|-----|------------------------|
| RFC-001 | Cryptographic identity as Root Keypair. State Chain as continuity proof. |
| RFC-001.5 | Identity Utility Model. M1 only needs storage replacement continuity. |
| R1 Architecture Reduction | M1 scope: storage replacement on Xiaomi Redmi Note 7. |

---

## 8. Decision

If RFC-002 is accepted:

1. **Continuity is defined** as Root Keypair match + verified State Chain.
2. **Boot architecture** follows the Genesis → Normal → Recovery model above.
3. **M1 scope** is the minimal continuity loop: generate identity, replace storage, recover identity, prove continuity.
4. **Recovery Seed** is the identity independence mechanism for M1.
5. **Identity on replaceable component** is an anti-pattern. Identity must be recoverable independently of the replaceable component.
6. **Open questions Q1-Q5** must be resolved before RFC-002 is finalized.

---

## 9. Next Steps

If RFC-002 is accepted:

1. Resolve open questions Q1-Q5.
2. Write specification: Boot Protocol — the interface between the bootloader and the identity system.
3. Write specification: Recovery Seed Format — the encoding and verification of the seed.
4. Begin M1 implementation: Genesis boot sequence on Xiaomi Redmi Note 7.

---

*End of RFC-002*
