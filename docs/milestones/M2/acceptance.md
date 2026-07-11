# M2 Acceptance Criteria: Environment Continuity Experiment

**Objective:** Prove that a computing system can survive storage replacement with both identity AND user environment intact.
**Method:** M1 continuity protocol + user data persistence verification.
**Platform:** QEMU (M2-A) → Xiaomi Redmi Note 7 (M2-B, deferred).
**Prerequisite:** M1-A is complete and accepted.

---

## 1. The Core Question

M1 proved:

> "I am the same system."

M2 must prove:

> "I am the same system AND my environment is intact."

The user does not experience a system as "theirs" solely because of a cryptographic keypair. The user experiences the system through:
- Their files and data
- Their configuration and settings
- Their installed applications
- Their session state

M2 extends the continuity model from **identity continuity** to **environment continuity**.

---

## 2. Experimental Setup

### 2.1 Architecture

```
M1: Identity Continuity Layer (validated)
    ├── Identity (Ed25519 keypair, seed-derived)
    ├── State Chain (signed documents)
    └── Recovery (BIP-39)

M2: Environment Continuity Layer (new)
    ├── User Data Store (files, configuration)
    ├── Environment Manifest (signed inventory of user environment)
    ├── Data Integrity Verification (hash-based)
    └── Migration Protocol (data + identity)
```

### 2.2 Key Insight

The State Document records the hardware inventory. M2 adds an **Environment Manifest** that records the software/user environment inventory:

```
EnvironmentManifest {
    system_public_key: String,
    sequence_number: u64,
    state_hash: String,              // Links to State Document
    user_data_hash: String,          // SHA-256 of user data
    configuration_hash: String,      // SHA-256 of /etc (or equivalent)
    software_inventory: Vec<SoftwareEntry>,
    timestamp: u64,
    signature: String,               // Signed by Root Private Key
}
```

Where `SoftwareEntry` records:
```
SoftwareEntry {
    name: String,
    version: String,
    hash: String,                     // Binary hash
    install_path: String,
}
```

The Environment Manifest is linked to the State Document. Both are signed by the same Root Keypair.

---

## 3. Experiment Sequence

### Phase 1: Genesis + Environment Creation

```
1. Boot system on Storage A
2. Generate identity (M1 protocol)
3. Create user data:
   - /home/user/documents/report.txt
   - /home/user/config/settings.json
   - /home/user/applications/notes.txt
4. Create Environment Manifest:
   - Hash user data
   - Record software inventory
   - Sign with Root Private Key
5. Record State Document (State[0]: genesis)
6. Boot normally
▶ Verify: identity exists, environment manifest signed, user data intact
```

### Phase 2: Normal Operation Verification

```
7. Boot on Storage A
8. Verify identity (M1 protocol)
9. Load Environment Manifest
10. Verify user data integrity (compare hash with manifest)
11. Boot normally
▶ Verify: environment manifest valid, data integrity confirmed
```

### Phase 3: Storage Death

```
12. Power off
13. Remove Storage A
14. Install Storage B (blank, no identity, no data)
```

### Phase 4: Recovery + Environment Continuity

```
15. Power on
16. Recovery mode (M1 protocol)
    - No identity found on Storage B
    - Recovery Seed entered
    - Keypair restored
    - Continuity verified against Genesis
17. User data restoration
    - Data from user backup (simulated) verified
    - Recovered data hash compared with Environment Manifest
18. Create Migration State Document (State[1]: storage_replaced)
19. Create Migration Environment Manifest
20. Boot normally
▶ Verify:
    - Identity continuity: ✅ (M1)
    - Environment continuity: ✅ (data hash matches manifest)
    - User data accessible: ✅
```

---

## 4. Acceptance Criteria

### 4.1 Must Prove ( ✅ Mandatory )

| # | Criterion | Verification Method |
|---|-----------|-------------------|
| C1 | Identity continuity (M1 criteria C1-C10) | M1 protocol |
| C2 | Environment Manifest is created and signed | Verify signature |
| C3 | User data hash matches Environment Manifest | Compare SHA-256 hashes |
| C4 | After storage replacement and recovery, user data hash still matches | Recover data, compare hash |
| C5 | Environment Manifest is linked to State Document | state_hash field matches |
| C6 | Full continuity: identity AND environment both verified | Combined check |
| C7 | External observer can verify environment continuity | Cryptographic verification |

### 4.2 Should Prove ( ✅ Recommended )

| # | Criterion | Importance |
|---|-----------|-----------|
| C8 | Partial data loss is detectable | Hash comparison detects corruption |
| C9 | Software inventory is recorded | Audit of installed components |
| C10 | Multiple files/directories are tracked | Realistic user data model |

### 4.3 Must NOT Prove ( ❌ Excluded )

| # | Non-Goal | Why Excluded |
|---|----------|-------------|
| N1 | Live data migration (no reboot required) | M2 operates on cold replacement |
| N2 | Incremental data synchronization | Full data set verification only |
| N3 | Filesystem driver development | Use host filesystem (QEMU) |
| N4 | Full OS kernel | Continuity layer only |
| N5 | Network-based recovery | Offline operation only |
| N6 | Encryption | Deferred to M3 |
| N7 | Versioned data history | Single version verification only |
| N8 | Application runtime state | Static file-level verification only |

---

## 5. M2-A Implementation Targets

| Component | Implementation | Notes |
|-----------|---------------|-------|
| Environment Manifest | New module: `src/environment.rs` | JSON format, signed |
| User data simulation | Test fixtures in `tests/data/` | Pre-created files + hashes |
| Data integrity check | SHA-256 comparison | Use existing `sha2` crate |
| Migration protocol | Extend `simulator.rs` | Add environment phase |
| Tests | 5+ new tests | Environment manifest, data integrity, migration |

---

## 6. M2 Success Definition

M2 is successful if and only if:

1. All M1 criteria pass (identity continuity).
2. All M2 mandatory criteria (C1-C7) pass.
3. The system can demonstrate the full Genesis → Death → Recovery cycle with user data verified intact.
4. An external observer can verify both identity AND environment continuity cryptographically.

M2 extends the continuity proof from:
```
"I am the same system."           (M1)
```
to:
```
"I am the same system with the same data."  (M2)
```

---

## 7. Relationship to M1

| Dimension | M1 | M2 |
|-----------|-----|-----|
| What is preserved | Identity (keypair + chain) | Identity + user environment |
| Data structure added | State Document | Environment Manifest |
| Hash verified | State chain hashes | User data hash |
| Storage scenario | Replace storage, recover identity | Replace storage, recover identity + data |
| User-visible result | "Same system" | "Same system, same data" |

---

## 8. After M2

If M2 succeeds, the next question becomes:

> How does the environment manifest interact with real filesystem operations?

This leads to M3: **Live Continuity** — monitoring filesystem changes and updating the environment manifest incrementally, rather than requiring a full re-hash at each state transition.

---

*End of M2 Acceptance Criteria*
