# M3-A Acceptance Criteria: Incremental Environment Continuity

**Objective:** Prove that the environment manifest can be updated incrementally (file changes → rehash → re-sign) without requiring full storage migration, and that the system can still prove continuity after storage death using the latest manifest.

**Method:** Independent environment manifest chain, incremental updates between state transitions.

**Platform:** QEMU (M3-A). M3-B (hardware) deferred.

**Prerequisite:** M2-A is complete and accepted.

---

## 1. The Core Question

M2 proved:

> "I am the same system with the same data." (static snapshot)

M3 must prove:

> "I am the same system with the same data — even after files changed between snapshots."

The practical gap M2 leaves open:

```
t0: Genesis, data=[report.txt, config.json]
    → Env[0], State[0]
t1: User edits report.txt
    → Env unchanged! Next re-hash only happens at storage migration.
t2: Storage death
    → Recovery restores OLD report.txt, verified against Env[0]
    → User loses their edits!
```

M3 solves this: **the environment manifest is live — it tracks changes as they happen.**

---

## 2. Architecture Change

### 2.1 Current (M2)

```
State Chain     Environment Chain
──────────────────────────────────
State[0] ◄────── Env[0]
(only updated on migration)
```

**Problem:** Environment is a static snapshot. Changes between migrations are invisible.

### 2.2 Target (M3)

```
State Chain     Environment Chain
──────────────────────────────────
State[0] ◄────── Env[0]
                 Env[1]  ← incremental update (file changed)
                 Env[2]  ← incremental update (file added)
State[1] ◄────── Env[3]  ← migration (storage replaced)
```

The environment chain lives independently of the state chain. The State Document links to the **latest** Environment Manifest at the time of its creation.

### 2.3 New Data Structure

Add `previous_env_hash` to EnvironmentManifest:

```rust
EnvironmentManifest {
    system_public_key: String,
    sequence_number: u64,
    previous_env_hash: String,      // NEW: links to previous Env in chain
    state_hash: String,             // links to current State Doc
    user_data_hash: String,
    configuration_hash: String,
    software_inventory: Vec<SoftwareEntry>,
    timestamp: u64,
    signature: String,
}
```

If `previous_env_hash` is `"0"*64`, this is the genesis manifest for this identity.

---

## 3. Experiment Sequence

### Phase 1: Genesis

```
1. Generate identity (existing)
2. Create user data: report.txt, config.json, notes.txt
3. Create Env[0]: hash user data, sign
4. Create State[0]: link to Env[0], sign
▶ Verify: Env[0] genesis, State[0] signed, links consistent
```

### Phase 2: Incremental Update (File Changed)

```
5. Modify report.txt (user edits content)
6. Create Env[1]:
   - previous_env_hash = hash(Env[0])
   - state_hash = hash(State[0])    // same State, no migration
   - Rehash user data (detects change)
   - Sign with Root Private Key
▶ Verify:
   - Env[1] signature valid
   - previous_env_hash links to Env[0]
   - user_data_hash differs from Env[0]
   - NO new State Document created
```

### Phase 3: Incremental Update (File Added)

```
7. Add new file: bookmarks.txt
8. Create Env[2]:
   - previous_env_hash = hash(Env[1])
   - state_hash = hash(State[0])
   - Rehash user data
   - Sign
▶ Verify: Env[2] chain valid, environment updated correctly
```

### Phase 4: Storage Death + Recovery

```
9. Power off
10. Remove Storage A
11. Install Storage B (blank)
12. Power on
13. Recovery mode:
    - Enter recovery seed
    - Keypair restored
    - Verify against Genesis
14. Restore user data (from backup)
15. Verify data against Env[2] (LATEST, not genesis)
16. Create State[1]: link to Env[3] (migration)
17. Create Env[3]: migration environment manifest
▶ Verify:
    - Identity continuity: ✅ (keypair match)
    - Environment continuity: ✅ (data matches LATEST Env)
    - User edits (t1) preserved: ✅
```

### Phase 5: Continuity Verification

```
18. Verify State Chain: [State[0], State[1]]
19. Verify Environment Chain: [Env[0], Env[1], Env[2], Env[3]]
20. Verify cross-links: State[0].env_hash == hash(Env[0]),
                        State[1].env_hash == hash(Env[3])
21. Verify data integrity: hash(recovered_data) == Env[3].user_data_hash
                        AND hash(recovered_data) includes t1 edits
▶ Continuity proven with incremental tracking: ✅
```

---

## 4. Acceptance Criteria

### 4.1 Must Prove ( ✅ Mandatory )

| # | Criterion | Verification Method |
|---|-----------|-------------------|
| C1 | Identity continuity (M1 criteria) | Keypair match + Chain valid |
| C2 | Environment manifest can be updated without state transition | Env[1] created, signed, no new State |
| C3 | Environment chain is independently verifiable | verify_environment_chain() on [Env[0], Env[1], Env[2]] |
| C4 | Modified data detected (hash differs from previous) | Env[1].user_data_hash != Env[0].user_data_hash |
| C5 | New files detected (hash changes) | Env[2].user_data_hash != Env[1].user_data_hash |
| C6 | Recovery uses LATEST environment manifest | Data verified against Env[2], not Env[0] |
| C7 | Cross-link consistency | State[n].environment_state_hash == hash(Env[m]) for the linked manifest |
| C8 | Full continuity: identity + incremental environment | Both chains valid, data intact |

### 4.2 Should Prove ( ✅ Recommended )

| # | Criterion | Importance |
|---|-----------|-----------|
| C9 | previous_env_hash chain is verifiable | Links between Envs are cryptographically sound |
| C10 | Recovered data matches final pre-death state | User edits are not lost |
| C11 | Multiple incremental updates (3+) are tracked | Chain length > 2, realistic usage |

### 4.3 Must NOT Prove ( ❌ Excluded )

| # | Non-Goal | Why Excluded |
|---|----------|-------------|
| N1 | Real filesystem watcher (inotify, FSEvents) | M3-A is simulated, not real-time |
| N2 | Automatic triggering of env updates | M3-A is manual/triggered updates |
| N3 | Conflict resolution for concurrent edits | Single-user simulation only |
| N4 | Versioned file history | Only latest hash tracked |
| N5 | Storage-efficient deltas | Full rehash per update (data is small) |
| N6 | Hardware boot chain | M3-B scope |
| N7 | Encryption at rest | Deferred to later milestone |

---

## 5. M3-A Implementation Plan

| Component | Change | New/Modified |
|-----------|--------|-------------|
| `environment.rs` | Add `previous_env_hash` field to EnvironmentManifest | Modified |
| `environment.rs` | Add `create_environment_update()` function | New |
| `environment.rs` | Update `canonical_manifest_string()` to include `previous_env_hash` | Modified |
| `environment.rs` | Update `verify_environment_chain()` to check `previous_env_hash` links | Modified |
| `environment.rs` | Add `hash_environment_chain()` helper | New |
| `simulator.rs` | Add Phase 2 (file change) and Phase 3 (file add) | Modified |
| `simulator.rs` | Update recovery to use latest Env instead of genesis | Modified |
| `simulator.rs` | Add `SimResult` fields for intermediate env hashes | Modified |

### 5.1 New Environment Manifest Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentManifest {
    pub system_public_key: String,
    pub sequence_number: u64,
    pub previous_env_hash: String,          // NEW
    pub state_hash: String,
    pub user_data_hash: String,
    pub configuration_hash: String,
    pub software_inventory: Vec<SoftwareEntry>,
    pub timestamp: u64,
    pub signature: String,
}
```

### 5.2 create_environment_update()

```rust
pub fn create_environment_update(
    previous_env: &EnvironmentManifest,
    state_hash: &str,
    user_data: &UserData,
) -> EnvironmentManifest
```

Creates a new Environment Manifest that:
- Inherits `system_public_key` from previous
- Increments `sequence_number`
- Sets `previous_env_hash = hash(previous_env)`
- Sets `state_hash` to current State Document hash
- Recomputes `user_data_hash` from current data
- Does NOT sign (signing is separate, same as State Documents)

### 5.3 Updated Environment Chain Verification

```rust
pub fn verify_environment_chain(manifests: &[EnvironmentManifest], data_set: &[UserData]) -> bool
```

Updated to check:
- `manifests[i].previous_env_hash == hash(manifests[i-1])` for i > 0
- All signatures valid
- All data hashes match

---

## 6. Success Definition

M3-A is successful if and only if:

1. All M1 and M2 mandatory criteria continue to pass.
2. All M3-A mandatory criteria (C1-C8) pass.
3. The full experiment demonstrates:
   - Genesis → File Change (Env updated independently) → File Add (Env updated again) → Storage Death → Recovery → Data verified against LATEST Env
4. Exit code 0 on `cargo run -- simulate`.

---

## 7. After M3-A

If M3-A succeeds, the next questions become:

> How does the environment manifest interact with real filesystem events (inotify, FSEvents)?
> How does this scale to thousands of files?

This leads to two possible branches:
- **M3-B**: Real filesystem watcher integration (event-driven updates)
- **M1-B**: Hardware port to Xiaomi Redmi Note 7

---

*End of M3 Acceptance Criteria*
