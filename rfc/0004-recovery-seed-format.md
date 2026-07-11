# RFC-004: Recovery Seed Format

**Status:** Draft for discussion
**Area:** Core Data Format
**Requires:** RFC-001 (Identity Model), RFC-002 (Bootstrap Architecture)
**Depends on RFCs:** RFC-001
**Supersedes:** Nothing
**Authors:** Theseus Architecture Team

---

## 1. Problem

The Recovery Seed is the mechanism by which a system's identity survives storage replacement. It must be:
- Human-transcribable (user can write it down)
- Machine-parseable (bootloader can read it)
- Cryptographically sound (generates the correct keypair)
- Error-tolerant (typos in transcription can be detected or corrected)

---

## 2. Format Selection: BIP-39

**Selected format:** BIP-39 mnemonic phrase.

BIP-39 is the standard for human-friendly cryptographic seed encoding, used by Bitcoin, Ethereum, and numerous hardware wallets. It is well-audited, widely implemented, and has excellent library support.

### 2.1 Why BIP-39

| Requirement | BIP-39 | Raw hex | QR code | Password |
|-------------|--------|---------|---------|----------|
| Human-transcribable | ✅ 12 words | ❌ 64 hex chars | ❌ Needs scanner | ❌ Can be forgotten |
| Error detection | ✅ Checksum word | ❌ No checksum | ✅ Built-in | ❌ None |
| Internationalization | ✅ 10+ languages | ❌ English only | ❌ Visual | ✅ Any |
| Standardization | ✅ BIP-39 spec | ❌ None | ✅ Various | ❌ None |
| Bootloader support | ⚠️ Needs parser | ✅ Trivial | ❌ Needs camera | ✅ Trivial |

### 2.2 Seed Length

| Length | Entropy | Words | M1 Decision |
|--------|---------|-------|-------------|
| 128-bit | 128 bits | 12 | **M1 default** |
| 192-bit | 192 bits | 18 | Future option |
| 256-bit | 256 bits | 24 | Future option |

**M1 uses 128-bit (12 word) seeds.** This provides adequate security for a proof of concept while minimizing user friction. Upgrading to 192 or 256 bits is a configuration change.

---

## 3. Root Keypair Derivation

### 3.1 From Seed to Keypair

```
BIP-39 Mnemonic (12 words)
        |
        ↓
BIP-39 Seed (512 bits, via PBKDF2)
        |
        ↓
Ed25519 Seed (256 bits, via SHA-512)
        |
        ├──→ Root Private Key (Ed25519 scalar)
        └──→ Root Public Key  (Ed25519 point)
```

### 3.2 Derivation Path

Multiple keypairs can be derived from a single BIP-39 seed using BIP-32 hierarchical derivation. For M1, a fixed derivation path is used:

```
m / 0' / 0' / 0'
```

This allows future expansion (different keys for different purposes) without changing the seed.

---

## 4. User Experience

### 4.1 First Boot (Genesis)

```
Screen display:

┌─────────────────────────────────────┐
│                                     │
│  ┌──────────────────────────────┐   │
│  │ SYSTEM IDENTITY SEED         │   │
│  │                              │   │
│  │ bridge  0x2c3f  mirror  72  │   │
│  │ river   0x9a1b  kiosk  15   │   │
│  │ forest  0x4d7e  cabin  88   │   │
│  │                              │   │
│  │ 📝 WRITE THIS DOWN           │   │
│  │ 🔐 KEEP IT SAFE              │   │
│  └──────────────────────────────┘   │
│                                     │
│  [ Continue ]  [ Show Again ]       │
│                                     │
└─────────────────────────────────────┘
```

The seed is displayed:
1. On screen during first boot
2. Written to `/boot/recovery.seed` for automated recovery

The user is prompted to:
1. Write down the 12 words
2. Store them in a safe place
3. Press "Continue" to proceed

### 4.2 Recovery

```
Screen display:

┌─────────────────────────────────────┐
│                                     │
│  SYSTEM IDENTITY RECOVERY           │
│                                     │
│  Storage device has changed.         │
│  Enter your recovery seed to        │
│  restore system identity.           │
│                                     │
│  Word 1: [________]                  │
│  Word 2: [________]                  │
│  ...                                 │
│  Word 12: [________]                 │
│                                     │
│  [ Verify ]  [ Skip (new identity) ]│
│                                     │
└─────────────────────────────────────┘
```

### 4.3 Seed Verification

After entry, the system:
1. Derives the Root Public Key from the entered seed
2. Compares it against the Genesis State Document's `system_public_key`
3. If match: identity restored, continuity proved
4. If no match: "This seed does not match this system's identity. Try again or start fresh."

---

## 5. Automated Recovery Path

For the M1 scenario where the old data partition is still accessible:

```
1. Boot from new storage
2. System checks for data partition from old storage
3. If found, look for /boot/recovery.seed
4. Found → automatically restore identity
5. Create Recovery State Document
6. Boot continues — user sees no recovery prompt
```

This is the zero-friction recovery path. The user only sees the recovery prompt if both the primary storage AND the data partition are lost.

---

## 6. Security Considerations

| Threat | Mitigation |
|--------|-----------|
| Seed stolen via physical access | Seed is encrypted on disk in M2+. For M1, seed is written in plaintext to `/boot/recovery.seed`. This is acceptable for a proof of concept. |
| Seed guessed by attacker | 128-bit entropy (12 words) provides 2^128 security. Not brute-forceable. |
| User loses seed | The system partition copy provides automated recovery. The external copy is user-managed. |
| Seed intercepted during display | The genesis display is on the local screen only. No network transmission. |
| Keypair derivation collision | BIP-39 + Ed25519 derivation is standard and collision-resistant. |

---

## 7. M1 Subset

| Feature | M1 Decision |
|---------|-------------|
| Seed length | 128-bit (12 words) |
| Word list | BIP-39 English |
| Derivation path | `m / 0' / 0' / 0'` |
| Automated recovery | ✅ From `/boot/recovery.seed` |
| Manual recovery | ✅ From keyboard entry |
| Encryption at rest | ❌ Seed stored in plaintext |
| Seed rotation | ❌ Single seed for lifetime |
| Multiple recovery keys | ❌ Not yet supported |
| Passphrase | ❌ Not yet supported |

---

## 8. Open Questions

| # | Question | Implications |
|---|----------|-------------|
| Q1 | Should the seed be encrypted on disk for M1? | Adds implementation complexity but protects against physical storage theft. |
| Q2 | Should the word list be localized? | BIP-39 supports 10+ languages. User preference. |
| Q3 | Should there be a QR code option for recovery? | Faster entry on mobile devices. Requires a camera. |

---

## 9. Decisions

1. **BIP-39 12-word mnemonic** is the Recovery Seed format for M1.
2. **Ed25519 keypair derivation** follows the BIP-32 path `m / 0' / 0' / 0'`.
3. **Automated recovery** from `/boot/recovery.seed` is the primary M1 path.
4. **Manual recovery** via keyboard entry is the fallback.
5. **Plaintext seed storage** is accepted for M1. Encryption is deferred.

---

## 10. Next Steps

If RFC-004 is accepted:

1. Choose or implement BIP-39 library for Rust (compatible with `no_std` for bootloader use).
2. Implement keypair derivation from seed.
3. Implement seed generation ceremony (first boot).
4. Implement seed verification during recovery.

---

*End of RFC-004*
