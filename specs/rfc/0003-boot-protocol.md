# RFC-003: Boot Protocol

**Status:** Draft for discussion
**Area:** Core Architecture
**Requires:** RFC-001 (Identity Model), RFC-002 (Bootstrap Architecture), RFC-005 (State Document Format)
**Depends on RFCs:** RFC-001, RFC-002, RFC-005
**Supersedes:** Nothing
**Authors:** Theseus Architecture Team

---

## 1. Problem

The boot protocol defines the sequence of events from power-on to a running system, with identity creation, verification, and recovery integrated into each stage. It answers:

- At what point during boot is the identity loaded?
- When are hardware changes detected?
- When is a new State Document created?
- What is the recovery sequence?
- What is the minimum bootable system?

---

## 2. Boot Stages

### Stage 0: Power-On / Bootloader

```
Entry:   Power applied, SoC boots
Exit:    Bootloader loaded, minimal hardware initialized
```

**Actions:**
1. SoC power-on sequence executes
2. Bootloader loads (U-Boot on Xiaomi Redmi Note 7)
3. Minimal hardware initialization (clock, memory, serial)
4. Storage subsystem initialized
5. Bootloader loads the Theseus boot image from storage

**Identity involvement:** None. Identity is not yet available in the bootloader stage for M1. The bootloader's only identity-related responsibility is to load the boot image.

**M1 Note:** On the Xiaomi Redmi Note 7, the bootloader chain is:
```
Qualcomm PBL → SBL → U-Boot → Theseus boot image
```
Theseus does not replace the Qualcomm boot chain. Theseus boots as an OS image loaded by U-Boot.

---

### Stage 1: Boot Image Entry

```
Entry:   Theseus boot image loaded into memory
Exit:    Root Keypair loaded, boot mode determined
```

**Actions:**
1. Minimal kernel environment initialized (MMU, interrupt controller, timer)
2. Storage driver loaded (eMMC driver for M1)
3. Storage partition table scanned
4. Look for `/theseus/state/genesis.cbor`
5. **If found:**
   - Load Root Keypair from keypair store
   - Load last State Document from `/theseus/state/`
   - Boot mode = NORMAL
6. **If NOT found:**
   - Boot mode = GENESIS

**Identity involvement:** Stage 1 determines whether identity exists. This is the critical branching point.

---

### Stage 2: Identity Resolution

```
Entry:   Boot mode determined
Exit:    Identity confirmed or created
```

#### Mode A: GENESIS

1. Generate Root Keypair (Ed25519)
2. Derive Recovery Seed (BIP-39, 12 words)
3. Generate State Document 0 (Genesis):
   ```
   State[0] = {
       system_public_key: RootPublicKey,
       sequence_number: 0,
       previous_state_hash: 00000000...,
       genesis_state_hash: content_hash_of_this_state,
       hardware_inventory: [current storage component],
       software_inventory: [],
       migration_reason: Genesis,
       timestamp: now(),
       content_hash: SHA256(all fields above),
       signature: Sign(RootPrivateKey, content_hash)
   }
   ```
4. Write State[0] to `/theseus/state/genesis.cbor`
5. Write Recovery Seed to:
   - Display on console
   - `/boot/recovery.seed`
6. Transition to Stage 4 (Boot)

#### Mode B: NORMAL

1. Load Root Keypair from keypair store
2. Load last State Document (State[N])
3. Collect current hardware inventory
4. Compare with State[N] hardware inventory
5. **If unchanged:**
   - No new State Document needed
   - Transition to Stage 4 (Boot)
6. **If changed:**
   - Determine MigrationReason
   - Create State[N+1]:
     ```
     State[N+1] = {
         ...
         sequence_number: N+1,
         previous_state_hash: SHA256(State[N]),
         genesis_state_hash: SHA256(State[0]),
         hardware_inventory: [current components],
         migration_reason: <detected change type>,
         ...
     }
     ```
   - Sign with Root Private Key
   - Write State[N+1] to `/theseus/state/state_N+1.cbor`
   - Transition to Stage 4 (Boot)

#### Mode C: RECOVERY

1. Look for data partition from previous storage
2. **If data partition found:**
   - Check `/boot/recovery.seed` on data partition
   - If found: automatic recovery (sub-mode C1)
   - If not found: prompt for seed (sub-mode C2)
3. **If data partition NOT found:**
   - Prompt for seed (sub-mode C2)

**Sub-mode C1: Automatic Recovery**

1. Read Recovery Seed from `/boot/recovery.seed`
2. Regenerate Root Keypair from seed
3. Load Genesis State Document from data partition
4. Verify: regenerated public key == Genesis `system_public_key`
5. Create Recovery State Document:
   ```
   State[N+1] = {
       ...
       migration_reason: RecoveryFromSeed,
       ...
   }
   ```
6. Write State[N+1] to new storage
7. Transition to Stage 4 (Boot)

**Sub-mode C2: Manual Recovery**

1. Display recovery prompt on console
2. User enters 12-word BIP-39 seed
3. Regenerate Root Keypair from seed
4. **If Genesis available** (on data partition):
   - Verify against Genesis
   - Create Recovery State Document
5. **If Genesis NOT available** (new system):
   - Create Genesis-from-recovery State Document:
     ```
     State[0]' = {
         ...
         migration_reason: RecoveryFromSeed,
         note: "Recovered from seed. Prior state unavailable."
     }
     ```
   - This is a new identity chain anchored by the same keypair
   - Continuity is cryptographic (same key) but not historical (no prior chain)
6. Transition to Stage 4 (Boot)

---

### Stage 3: Boot Decision (Safety Check)

```
Entry:   Identity resolved
Exit:    Boot continues OR halt for manual intervention
```

**Decision logic:**

```
if identity_state == CONTINUITY_PROVEN:
    boot()
elif identity_state == CONTINUITY_LOST:
    if recovery_seed_available():
        recover()
        boot()
    else:
        halt("System identity lost. Enter recovery seed to restore.")
elif identity_state == NEW_SYSTEM:
    genesis()
    boot()
```

**Safety rule:** A system must never boot in an unknown identity state. If identity cannot be resolved, the system halts with a clear message.

---

### Stage 4: System Boot

```
Entry:   Identity confirmed
Exit:    Running system
```

**Actions:**
1. Initialize remaining subsystems (filesystem, networking, etc.)
2. Mount root filesystem
3. Start init process
4. System is operational

**Identity involvement:** The system identity is now available via a system interface (e.g., `/theseus/identity/public_key`). Applications and services can query the identity.

---

## 3. Boot Mode State Machine

```
                  Power On
                     |
                     v
              ┌───────────────┐
              │ Stage 0       │
              │ Bootloader    │
              └───────┬───────┘
                      |
                      v
              ┌───────────────┐
              │ Stage 1       │
              │ Identity      │
              │ Check         │
              └───────┬───────┘
                      |
          ┌───────────┼───────────┐
          |           |           |
          v           v           v
   ┌───────────┐ ┌─────────┐ ┌──────────┐
   │ GENESIS   │ │ NORMAL  │ │ RECOVERY │
   └─────┬─────┘ └────┬────┘ └─────┬────┘
         |            |            |
         v            v            v
   ┌───────────┐ ┌─────────┐ ┌──────────┐
   │ Generate  │ │ Verify  │ │ Restore  │
   │ Identity  │ │ Chain   │ │ Identity │
   └─────┬─────┘ └────┬────┘ └─────┬────┘
         |            |            |
         └────────────┼────────────┘
                      |
                      v
              ┌───────────────┐
              │ Stage 3       │
              │ Boot Decision │
              └───────┬───────┘
                      |
                      v
              ┌───────────────┐
              │ Stage 4       │
              │ System Boot   │
              └───────────────┘
```

---

## 4. Identity Interface

Once booted, the system exposes its identity through a simple interface:

### 4.1 Kernel Interface

```
/sys/theseus/identity/
├── public_key        (read)    Root Public Key (hex)
├── genesis_hash      (read)    Genesis State Document hash
├── state_count       (read)    Number of State Documents
├── current_state     (read)    Current State Document hash
├── continuity        (read)    "proven" | "recovered" | "genesis"
└── recovery_seed     (write)   Trigger seed recovery workflow
```

### 4.2 Userland Tool

```
theseus-identity
  status          -- Show identity status
  verify          -- Verify State Chain integrity
  export          -- Export current identity (public key)
  recover         -- Enter recovery workflow
  show-seed       -- Display Recovery Seed (requires confirmation)
```

---

## 5. M1 Boot Sequence (Complete)

The full M1 boot sequence, from power-on to identity verification:

```
Power On
  │
  ▼
U-Boot loads Theseus boot image from eMMC
  │
  ▼
Minimal kernel init (MMU, interrupts, timer)
  │
  ▼
eMMC driver loaded
  │
  ▼
Scan partition table
  │
  ▼
Look for /theseus/state/genesis.cbor
  │
  ├── Found? → Load identity → Compare hardware
  │               │
  │               ├── Unchanged? → Boot
  │               │
  │               └── Changed? → New State Doc → Boot
  │
  └── Not found? → GENESIS mode
                      │
                      ├── Generate keypair
                      ├── Display seed
                      ├── Create Genesis State
                      ├── Write to storage
                      └── Boot
```

---

## 6. M1 Subset

| Feature | M1 Decision |
|---------|-------------|
| Boot stages | Stages 0-4 as defined |
| Boot modes | GENESIS, NORMAL (unchanged), RECOVERY (seed) |
| Hardware change detection | Storage only |
| Automatic recovery | From `/boot/recovery.seed` on data partition |
| Manual recovery | Keyboard entry on console |
| Bootloader integration | Minimal (U-Boot loads image, Theseus handles identity) |
| Safety halt on lost identity | ✅ Yes |
| `/sys/theseus/identity/` interface | ✅ Yes |
| `theseus-identity` userland tool | ✅ Yes |

---

## 7. Open Questions

| # | Question | Implications |
|---|----------|-------------|
| Q1 | Should the bootloader verify the identity before loading the OS image? | Adds security but increases bootloader complexity. Deferred to M2+. |
| Q2 | What if the State Chain is corrupted? | The system cannot prove continuity. Falls back to recovery mode. |
| Q3 | Should the system support booting without identity (ephemeral mode)? | Useful for debugging/triage. But violates identity-first architecture. |
| Q4 | How does the boot protocol interact with encrypted storage? | Encryption key could be derived from the identity. Deferred. |

---

## 8. Decisions

1. **Boot protocol** follows the 5-stage model: Bootloader → Identity Check → Identity Resolution → Boot Decision → System Boot.
2. **Identity resolution** (Stage 2) is the architectural core of the boot process.
3. **Safety halt** on identity loss is mandatory. No booting in unknown state.
4. **M1 implements** GENESIS, NORMAL (storage change detection), and RECOVERY modes.
5. **Bootloader identity verification** is deferred.
6. **Ephemeral mode** (boot without identity) is explicitly rejected for M1.

---

## 9. Next Steps

If RFC-003 is accepted:

1. Develop the boot sequence as a Rust implementation targeting QEMU (M1-A: simulator).
2. Implement Stage 1-3 identity logic (no hardware dependencies).
3. Implement Stage 2 hardware change detection for storage.
4. Implement Stage 2 recovery workflow.
5. Port to Xiaomi Redmi Note 7 hardware (M1-B: real hardware).

---

*End of RFC-003*
