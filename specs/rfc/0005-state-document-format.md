# RFC-005: State Document Format

**Status:** Draft for discussion
**Area:** Core Data Format
**Requires:** RFC-001 (Identity Model), RFC-002 (Bootstrap Architecture)
**Depends on RFCs:** RFC-001, RFC-002
**Supersedes:** Nothing
**Authors:** Theseus Architecture Team

---

## 1. Problem

The State Document is the fundamental data structure of the Theseus identity system. Every hardware change, every migration, and every continuity proof depends on it. This RFC defines the format, contents, and lifecycle of State Documents.

---

## 2. Format Definition

### 2.1 Logical Structure

```
StateDocument {
    // Identity
    system_public_key: PublicKey,

    // Chain position
    sequence_number: Uint64,
    previous_state_hash: Hash256,
    genesis_state_hash: Hash256,

    // Content
    hardware_inventory: HardwareInventory,
    software_inventory: SoftwareInventory,
    migration_reason: MigrationReason,
    timestamp: UnixTimestamp,

    // Proof
    content_hash: Hash256,
    signature: Signature,
}
```

### 2.2 Field Semantics

| Field | Type | Description |
|-------|------|-------------|
| `system_public_key` | Ed25519 public key | The system's canonical identity. Must be identical across all State Documents in the same chain. |
| `sequence_number` | Uint64 | Monotonically increasing. 0 = genesis. Each subsequent State Document increments by 1. |
| `previous_state_hash` | SHA-256 | `SHA256(previous State Document)` — links to the prior state. For genesis, all zeros. |
| `genesis_state_hash` | SHA-256 | `SHA256(genesis State Document)` — constant across the entire chain. Enables fast chain verification without traversing every state. |
| `hardware_inventory` | Component[] | Complete list of hardware components detected at this state. |
| `software_inventory` | SoftwareComponent[] | Complete list of software/OS components at this state. |
| `migration_reason` | Enum | Why this State Document was created (see section 3). |
| `timestamp` | Unix timestamp | When this State Document was created. Best-effort accuracy; not relied upon for cryptographic verification. |
| `content_hash` | SHA-256 | `SHA256(all fields except content_hash and signature)` — the digest that is signed. |
| `signature` | Ed25519 signature | `Sign(RootPrivateKey, content_hash)` — proves authenticity. |

### 2.3 HardwareInventory Entry

```
HardwareComponent {
    component_class: ComponentClass,  // See 2.3.1
    vendor_id: String,
    model_id: String,
    serial_number: String (optional),
    firmware_version: String (optional),
    component_identity: Hash256,       // SHA256(class + vendor + model + serial)
    status: ComponentStatus,           // present, removed, replaced
}
```

#### 2.3.1 Component Classes

| Class | Examples | M1 Required? |
|-------|----------|--------------|
| `storage` | eMMC, NVMe, SD card | ✅ Yes |
| `cpu` | Snapdragon 660, ARM Cortex | No |
| `memory` | LPDDR4, DDR5 | No |
| `gpu` | Adreno, Mali | No |
| `display` | LCD panel, OLED | No |
| `network` | Wi-Fi, Bluetooth, modem | No |
| `motherboard` | Mainboard, SoC package | No |
| `bootloader` | U-Boot, Limine | No |
| `firmware` | UEFI, device tree | No |

For M1, only `storage` is required. All other classes are optional.

### 2.4 SoftwareInventory Entry

```
SoftwareComponent {
    component_name: String,
    version: String,
    hash: Hash256,                     // SHA256 of the component binary
    component_type: SoftwareType,      // kernel, driver, system_service, library
}
```

For M1, software inventory is informational. It is recorded but not required for continuity verification.

### 2.5 MigrationReason

```
MigrationReason : enum {
    Genesis:              // First boot, no prior state
    StorageReplaced,      // Storage component changed
    ComponentAdded,       // New hardware component detected
    ComponentRemoved,     // Hardware component removed
    ComponentReplaced,    // Hardware component swapped
    RecoveryFromSeed,     // Identity restored from Recovery Seed
    PlannedMigration,     // User-initiated hardware migration
    UnscheduledBoot,      // Boot after unexpected power loss (unknown state)
}
```

For M1, only `Genesis`, `StorageReplaced`, and `RecoveryFromSeed` are required.

---

## 3. Serialization Format

### 3.1 Wire Format

State Documents are serialized using a deterministic, self-describing format:

```
Format: CBOR (Concise Binary Object Representation)

Rationale:
- Deterministic encoding (canonical CBOR) ensures hashes are reproducible.
- Compact binary format (smaller than JSON, XML).
- Self-describing (no external schema required for basic parsing).
- Sufficient for embedded/bootloader environments (minimal parsing code).
```

### 3.2 File Storage

State Documents are stored as individual files on the system partition:

```
/theseus/state/
├── genesis.cbor          # State 0
├── state_000001.cbor     # State 1
├── state_000002.cbor     # State 2
└── state_000003.cbor     # State N (current)
```

### 3.3 Chain Verification

To verify chain integrity:

```
verify_chain(states):
    if states is empty: return FAILURE

    // Verify genesis
    genesis = states[0]
    if genesis.sequence_number != 0: return FAILURE
    if genesis.previous_state_hash != zeros: return FAILURE
    if !verify_signature(genesis): return FAILURE
    genesis_hash = SHA256(genesis)

    // Verify each subsequent state
    for i = 1 to len(states)-1:
        state = states[i]
        if state.sequence_number != i: return FAILURE
        if state.genesis_state_hash != genesis_hash: return FAILURE
        if state.previous_state_hash != SHA256(states[i-1]): return FAILURE
        if !verify_signature(state): return FAILURE

    return SUCCESS
```

---

## 4. State Document Lifecycle

### 4.1 Genesis

```
Event: First boot
Action:
  1. Generate Root Keypair
  2. Collect initial hardware inventory (minimal: storage identity)
  3. Create Genesis State Document (sequence_number: 0)
  4. Sign with Root Private Key
  5. Write to /theseus/state/genesis.cbor
```

### 4.2 Normal Update

```
Event: Hardware change detected during boot
Action:
  1. Collect current hardware inventory
  2. Compare with previous State Document's inventory
  3. If changed:
       a. Determine MigrationReason
       b. Create new State Document (sequence_number: previous + 1)
       c. Sign with Root Private Key
       d. Write to /theseus/state/state_NNNNNN.cbor
```

### 4.3 Recovery Update

```
Event: Storage replaced, identity recovered from seed
Action:
  1. Regenerate Root Keypair from Recovery Seed
  2. Verify against Genesis State Document (check public key match)
  3. Collect current hardware inventory
  4. Create Recovery State Document (sequence_number: previous + 1)
       - migration_reason: RecoveryFromSeed
  5. Sign with Root Private Key
  6. Write to /theseus/state/state_NNNNNN.cbor
```

---

## 5. M1 Subset

For M1, the State Document is minimized:

| Field | M1 Required? |
|-------|-------------|
| `system_public_key` | ✅ Yes |
| `sequence_number` | ✅ Yes |
| `previous_state_hash` | ✅ Yes |
| `genesis_state_hash` | ✅ Yes |
| `hardware_inventory` | ✅ Yes (only `storage` class required) |
| `software_inventory` | ❌ No (empty list) |
| `migration_reason` | ✅ Yes |
| `timestamp` | ✅ Yes (informational) |
| `content_hash` | ✅ Yes |
| `signature` | ✅ Yes |

---

## 6. Open Questions

| # | Question | Implications |
|---|----------|-------------|
| Q1 | Should State Documents be encrypted? | If stored on the system partition, anyone with physical access can read the hardware evolution history. |
| Q2 | Should State Documents be signed with a timestamp authority? | Enables external verification of "this state existed at this time." Adds network dependency. |
| Q3 | How large can a State Document get? | Hardware inventory could grow large on complex systems. Needs a size limit or truncation strategy. |
| Q4 | Should the State Chain support pruning? | Old states could be archived to reduce storage. Conflicts with audit completeness. |

---

## 7. Decisions

1. **State Document format** is CBOR with the fields specified above.
2. **Chain verification** follows the algorithm in section 3.3.
3. **M1 uses only the storage class** in hardware inventory. All other classes are deferred.
4. **Open questions Q1-Q4** are deferred until after M1.

---

## 8. Next Steps

If RFC-005 is accepted:

1. Write the CBOR schema specification for State Documents.
2. Implement State Document creation and verification in Rust.
3. Integrate with the boot protocol (RFC-003).

---

*End of RFC-005*
