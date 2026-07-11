# RFC-006: Environment Continuity Model

| Field | Value |
|-------|-------|
| **Status** | Accepted |
| **Replaces** | — |
| **Depends on** | RFC-001 (Identity Model), RFC-005 (State Document Format) |
| **Validated by** | M2-A Experiment |

---

## 1. Summary

RFC-006 defines the **Environment Continuity Model**: the data structures and protocols that allow a Theseus system to prove that its user environment (files, configuration, software) survives hardware replacement.

This is the second layer of the Theseus Continuity Protocol, built on top of the Identity Model (RFC-001).

---

## 2. Motivation

RFC-001–005 define how a system proves **identity continuity**:

> "I am the same system."

But a user experiences a system through its environment:
- Files and documents
- System and application configuration
- Installed software
- Session state

Without environment continuity, identity continuity is abstract. The user can prove "this is the same keypair" but cannot prove "this is the same data."

RFC-006 formalizes how environment continuity is created, verified, and migrated.

---

## 3. Data Structures

### 3.1 Environment Manifest

```rust
EnvironmentManifest {
    system_public_key: String,      // Ed25519 public key (links to identity)
    sequence_number: u64,           // Monotonic counter within identity
    state_hash: String,             // SHA-256 of the linked State Document
    user_data_hash: String,         // SHA-256 of all tracked user data
    configuration_hash: String,     // SHA-256 of system configuration
    software_inventory: Vec<SoftwareEntry>,
    timestamp: u64,
    signature: String,              // Signed by the Root Private Key
}
```

The Environment Manifest is the peer to the State Document. The State Document tracks hardware; the Environment Manifest tracks software and data.

### 3.2 Software Entry

```rust
SoftwareEntry {
    name: String,                   // Package/application name
    version: String,                // Version string
    hash: String,                   // SHA-256 of the binary
    install_path: String,           // Filesystem path
}
```

### 3.3 User Data Model

```rust
UserData {
    files: BTreeMap<String, String>, // path → content
}
```

User data is represented as a map of filesystem paths to content. The hash is computed deterministically:

```
hash = SHA-256(path[0] + \x00 + content[0] + \x00 + ... + path[n] + \x00 + content[n] + \x00)
```

The path-content pairs are sorted by path (via BTreeMap) for deterministic ordering.

---

## 4. Linking Mechanism

The State Document and Environment Manifest are linked bidirectionally:

```
StateDocument {
    ...
    environment_state_hash: String,  // SHA-256 of the linked EnvironmentManifest
    ...
}

EnvironmentManifest {
    ...
    state_hash: String,              // SHA-256 of the linked StateDocument (or "0"*64 for genesis)
    ...
}
```

This creates a cryptographic cross-link:

```
State[0] ◄────env_hash────► Env[0]
   │                              │
   │                              │
State[1] ◄────env_hash────► Env[1]
```

Both chains must verify independently, and the links must be consistent.

---

## 5. Lifecycle

### 5.1 Genesis

```
1. Generate identity seed
2. Derive Ed25519 keypair
3. Create user data
4. Create EnvironmentManifest[0]:
   - Hash user data
   - Record software inventory
   - Set state_hash = "0"*64 (pre-linking)
   - Sign with Root Private Key
5. Create StateDocument[0]:
   - Record hardware inventory
   - Set environment_state_hash = hash(Env[0])
   - Sign with Root Private Key
```

State[0] is created AFTER Env[0], so it can reference it. The Env[0] pre-links with a placeholder.

### 5.2 Normal Operation

```
1. Verify identity (from keypair)
2. Load StateDocument[n]
3. Load EnvironmentManifest[n]
4. Verify both signatures
5. Verify link: State[n].environment_state_hash == hash(Env[n])
6. Verify data integrity: hash(user_data) == Env[n].user_data_hash
7. Boot
```

### 5.3 Migration (Storage Replacement)

```
1. Detect: no identity on new storage → Recovery mode
2. Enter recovery seed
3. Regenerate keypair
4. Load Genesis State and Environment Manifest (from backup)
5. Verify identity: restored keypair matches Genesis
6. Restore user data from backup
7. Create EnvironmentManifest[n+1]:
   - Hash restored user data
   - Set state_hash = hash(State[n])   // links to most recent StateDocument
   - Sign with Root Private Key
8. Create StateDocument[n+1]:
   - Record new hardware inventory
   - Set environment_state_hash = hash(Env[n+1])
   - Sign with Root Private Key
9. Verify chain: State[0..n+1] + Env[0..n+1]
10. Verify data: hash(restored_data) == Env[0].user_data_hash
11. Continuity proven → Boot
```

---

## 6. Verification Rules

1. Every Environment Manifest must be signed by the Root Private Key.
2. The hash in `user_data_hash` must match the SHA-256 of the actual user data at creation time.
3. The `state_hash` in Env[n] must match the hash of the StateDocument it was created to accompany, OR be `"0"*64` for the genesis manifest.
4. The `environment_state_hash` in State[n] must match the hash of the EnvironmentManifest[n].
5. For continuity to be proven, both the identity chain (State Documents) AND the environment chain (Environment Manifests) must verify, AND the recovered user data hash must match the genesis `user_data_hash`.

---

## 7. Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Manifest format | JSON (M2) | Same as State Documents. CBOR deferred. |
| Hash function | SHA-256 | Same as State Documents. Consistent with Ed25519 security level. |
| Data model | Path-content map | Sufficient for M2 proof. Real filesystem integration is M3 scope. |
| Software inventory | Static entries | M2 only needs proof of concept. Dynamic inventory is M3 scope. |
| User data granularity | Full-directory hash | M2 verifies entire data set. Incremental updates are M3 scope. |

---

## 8. Open Questions

| Question | Status |
|----------|--------|
| How does the manifest interact with real filesystem write events? | M3-A target |
| Should deleted files be tracked? | M3-A target |
| How large can the manifest grow? | Deferred |
| Is software inventory needed for continuity proof or only for audit? | Deferred |
| Should the configuration hash include all of /etc or only tracked files? | M3-A target |

---

## 9. References

- RFC-001: Identity Model (keypair and seed architecture)
- RFC-005: State Document Format (hardware inventory and chain)
- M2-A Experiment: Implementation validating this model
- M2_ACCEPTANCE_CRITERIA.md: Acceptance criteria for the M2-A experiment

---

*End of RFC-006*
