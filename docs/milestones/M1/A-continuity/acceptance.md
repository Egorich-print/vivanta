# M1 Acceptance Criteria: Continuity Proof Experiment

**Objective:** Prove that a computing system can survive the death of its primary storage and remain itself.
**Method:** Genesis → storage replacement → recovery → continuity verification.
**Platform:** QEMU (M1-A), Xiaomi Redmi Note 7 / lavender (M1-B).
**Scope:** Theseus Continuity Layer only. No kernel, no drivers, no UI, no applications.

---

## 1. The Core Thesis

M1 exists to answer one question:

> **Can a digital entity survive replacement of its physical substrate and prove it is the same entity?**

This is not a storage migration test. This is a **continuity proof experiment**. The mechanism is storage replacement. The goal is identity continuity.

---

## 2. Experimental Setup

### 2.1 Components

| Component | Role |
|-----------|------|
| Theseus Identity Runtime | Generates Root Keypair, manages State Chain, handles recovery |
| Virtual storage device (M1-A) / eMMC (M1-B) | The replaceable component |
| Recovery Seed (BIP-39) | Identity survival mechanism |
| State Documents (CBOR) | Chain of hardware states |

### 2.2 Sequence

```
Phase 1: Genesis
  1. Boot system on Storage A
  2. System generates Root Keypair (Ed25519)
  3. System displays Recovery Seed (12 BIP-39 words)
  4. System creates Genesis State Document (State[0])
  5. System writes user data to Storage A
  6. System boots normally
  ▶ Verify: identity exists, chain starts, seed recorded

Phase 2: Storage Replacement
  7. Power off
  8. Remove Storage A
  9. Install Storage B (blank, no identity)

Phase 3: Recovery
  10. Power on
  11. System detects no identity on Storage B
  12. System enters Recovery mode
  13. Recovery Seed entered (automated from backup or manual)
  14. System regenerates Root Keypair from seed
  15. System verifies: recovered keypair matches Genesis State[0].system_public_key
  ▶ Verify: continuity proven cryptographically

Phase 4: Continuity Verification
  16. System creates Recovery State Document (State[1])
  17. State Chain: State[0] → State[1]
  18. System boots normally
  19. System exposes identity via /identity/public_key
  ▶ Verify: same identity before and after storage replacement
```

---

## 3. Acceptance Criteria

### 3.1 Must Prove ( ✅ Mandatory )

| # | Criterion | Verification Method |
|---|-----------|-------------------|
| C1 | Genesis creates a valid Root Keypair | Inspect generated public key |
| C2 | Recovery Seed can regenerate the identical keypair | Compare public key from seed vs. original |
| C3 | Genesis State Document is created and signed | Verify signature with Root Public Key |
| C4 | System boots normally after Genesis | Expected boot sequence completes |
| C5 | Storage replacement is detected (no identity on new storage) | System enters Recovery mode |
| C6 | Recovery Seed restores identity successfully | Regenerated keypair matches Genesis public key |
| C7 | Recovery State Document is created and signed, referencing Genesis | State[1].genesis_state_hash == SHA256(State[0]) |
| C8 | State Chain verifies: State[0] → State[1] chain is valid | Chain verification algorithm returns SUCCESS |
| C9 | System exposes identity after recovery | `/identity/public_key` is readable and matches original |
| C10 | Continuity is formally proven | Root Keypair matches AND State Chain is verified |

### 3.2 Should Prove ( ✅ Recommended )

| # | Criterion | Importance |
|---|-----------|-----------|
| C11 | Recovery from automated seed file (`/boot/recovery.seed`) | Primary M1 recovery path |
| C12 | Recovery from manual seed entry (keyboard) | Fallback recovery path |
| C13 | System creates State Document on any hardware change | Future extensibility |
| C14 | State Chain integrity check detects tampering | Security foundation |

### 3.3 Must NOT Prove ( ❌ Excluded )

| # | Non-Goal | Why Excluded |
|---|----------|-------------|
| N1 | Hot hardware replacement (no reboot) | M1 operates on cold replacement. Hotplug is a future concern. |
| N2 | CPU/GPU/motherboard replacement | M1 tests storage only. Architecture independence starts here. |
| N3 | Operating system kernel | M1 does not implement a kernel. It proves continuity. |
| N4 | Desktop or mobile UI | No display stack. Serial console only. |
| N5 | Application runtime | No applications exist yet. |
| N6 | Linux/Android compatibility | No compatibility layer. |
| N7 | Networking | No network stack. No network-based recovery. |
| N8 | Security hardening | Plaintext seed on disk is accepted for M1. |
| N9 | Performance optimization | Correctness over performance. |
| N10 | Multi-device identity | Single device only. |
| N11 | Fork detection | Single system. No clones. |
| N12 | Ownership transfer | Single user. No transfer. |
| N13 | Encrypted storage | No encryption in M1. |
| N14 | TPM/secure element integration | Software-only key storage. |
| N15 | Self-modification | No Adaptive Engine. No runtime reconfiguration. |

---

## 4. Architectural Boundaries

### 4.1 Theseus Continuity Layer (M1 Scope)

```
┌─────────────────────────────────────┐
│           THESEUS CONTINUITY        │
│                                     │
│  - Identity Generation              │
│  - State Chain Management           │
│  - Recovery Protocol                │
│  - Boot Mode Engine                 │
│  - Identity Interface               │
│                                     │
│  Depends on: storage driver         │
│  Provides: identity, continuity     │
└─────────────────────────────────────┘
```

### 4.2 Operating System Layer (Not M1 Scope)

```
┌─────────────────────────────────────┐
│           OPERATING SYSTEM          │
│                                     │
│  - Kernel / Scheduler               │
│  - Memory Manager                   │
│  - Filesystem                       │
│  - Device Drivers                   │
│  - Network Stack                    │
│  - Application Runtime              │
│  - UI Framework                     │
│                                     │
│  Depends on: continuity layer       │
│  Provides: full OS functionality    │
└─────────────────────────────────────┘
```

The Theseus Continuity Layer is not an OS. It is a **protocol layer** that an OS can be built on top of. M1 validates this layer. If M1 succeeds, the OS layer becomes a future project.

---

## 5. M1-A (QEMU) vs. M1-B (Hardware)

### M1-A: QEMU Simulation

| Dimension | Detail |
|-----------|--------|
| **Platform** | QEMU emulating ARM64 |
| **Storage** | Virtual disk image (qcow2) |
| **Replacement** | Swap qcow2 file, boot from new image |
| **Recovery** | Seed file on separate virtual media |
| **Complexity** | Low |
| **Risk** | Minimal |
| **Deliverable** | Continuity protocol verified |

### M1-B: Xiaomi Redmi Note 7

| Dimension | Detail |
|-----------|--------|
| **Platform** | Xiaomi Redmi Note 7 (lavender) |
| **Storage** | Physical eMMC |
| **Replacement** | Desolder/reseat eMMC or use SD card as secondary storage |
| **Recovery** | Seed file on SD card or keyboard entry via UART |
| **Complexity** | High (boot chain, Qualcomm secure boot, device tree) |
| **Risk** | Medium (hardware boot chain complexity) |
| **Deliverable** | Continuity protocol on real hardware |

### Decision

M1-A is **mandatory** before M1-B. The continuity protocol must be proven in simulation before incurring hardware boot chain complexity.

---

## 6. Failure Modes

| Mode | Symptom | Response |
|------|---------|----------|
| Keypair mismatch after recovery | Regenerated keypair does not match Genesis | Seed restoration is incorrect. Check derivation path. |
| Chain verification failure | State[1] does not link to State[0] | State Document format error. Check hash linkage. |
| Recovery mode not entered | System boots on new storage without detecting identity loss | Identity detection logic is broken. Check boot mode engine. |
| Seed file not found | Automated recovery fails | Seed file path or format is incorrect. |
| Boot chain failure (M1-B) | System does not boot on hardware | Bootloader configuration or hardware initialization issue. |

---

## 7. M1 Success Definition

M1 is successful if and only if:

1. All mandatory criteria (C1-C10) pass.
2. The system can demonstrate the full Genesis → Replacement → Recovery → Continuity loop.
3. An external observer can verify continuity without trusting the system (cryptographic verification).

M1 is NOT considered successful if:
- The protocol works but cannot be externally verified.
- The protocol works in QEMU but cannot be ported to hardware.
- The protocol works but requires manual intervention at every step (recovery must be scriptable).

---

## 8. Decision

The following decisions are accepted:

1. M1 is a **Continuity Proof Experiment**, not an OS implementation.
2. M1 scope is strictly bounded by the 15 non-goals (N1-N15).
3. M1-A (QEMU) must succeed before M1-B (hardware).
4. Success is defined by criteria C1-C10.
5. The Theseus Continuity Layer and the Operating System Layer are architecturally separated.
6. All future milestones are deferred until M1 is accepted.

---

*End of M1 Acceptance Criteria*
