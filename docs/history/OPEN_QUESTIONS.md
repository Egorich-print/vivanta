# Open Questions

This document tracks unresolved architectural questions for the Vivanta project. Questions are resolved
through RFCs or deferred to later milestones.

## Recently Resolved Questions

These questions were addressed by the RFC chain and are now closed:

| Question | Resolution | RFC |
|----------|-----------|-----|
| What is system identity? | Cryptographic keypair (Ed25519) | RFC-001 |
| How is continuity proven? | State Chain: signed documents linking genesis to current state | RFC-001 |
| Why does identity matter? | 8 of 10 analyzed capabilities require identity; core value is transforming hardware replacement from catastrophic to routine | RFC-001.5 |
| What is the bootstrap architecture? | 3 boot modes: Genesis, Normal, Recovery | RFC-002 |
| What is "the same system"? | Same Root Keypair + verified State Chain | RFC-002 |
| How does the boot sequence work? | 5-stage protocol: Bootloader → Identity Check → Identity Resolution → Boot Decision → System Boot | RFC-003 |
| What is the Recovery Seed format? | BIP-39 12-word mnemonic | RFC-004 |
| What is the State Document format? | CBOR with specific fields (system_public_key, sequence_number, previous_state_hash, hardware_inventory, signature, etc.) | RFC-005 |
| How is identity independence achieved? | Recovery Seed is stored independently of the replaceable component | RFC-002, RFC-004 |

## Open Questions Requiring Resolution Before M1 Implementation

| Priority | Question | Why It Matters | Blocks | Possible Approaches | Owner | Status |
|----------|----------|---------------|-------|---------------------|-------|--------|
| HIGH | Should the Recovery Seed be encrypted on disk for M1? | Plaintext seed on `/boot/recovery.seed` is a security concern even for M1. | M1 implementation | (a) Plaintext (simplest), (b) Encrypted with a fixed key, (c) Encrypted with user passphrase | TBD | Open |
| HIGH | Where should the State Chain be stored: system partition or dedicated partition? | If on system partition, it dies with storage. If dedicated, it could survive. | M1 storage layout | (a) System partition (simpler), (b) Dedicated identity partition, (c) Data partition | TBD | Open |
| MEDIUM | Should M1 support the "no Genesis available" recovery path? | If the data partition is lost and only the seed remains, the system has the keypair but no prior state. | Recovery flow design | (a) Create new genesis (simpler), (b) Require Genesis for continuity (stricter) | TBD | Open |
| MEDIUM | What entropy source is used for keypair generation on first boot? | Deterministic keypair generation depends on hardware RNG quality. | Genesis implementation | (a) CPU RNG, (b) Combined sources, (c) Bootloader-provided entropy | TBD | Open |

## Deferred Questions (Past M1)

These questions are recognized as important but are intentionally deferred until after M1 delivers the minimal
continuity proof.

| Priority | Question | Expected Impact | Proposed RFC |
|----------|----------|----------------|--------------|
| LOW | How does fork detection work? | Detecting clones requires coordination between systems. Post-M1. | RFC-006 |
| LOW | How does ownership transfer work? | Changing ownership without changing identity. Post-M1. | RFC-007 |
| LOW | Should the bootloader verify identity? | Security hardening. Post-M1. | RFC-008 |
| LOW | What is the encrypted storage model? | Key derivation from system identity. Post-M1. | RFC-009 |
| LOW | How do multi-device trust relationships work? | Device-to-device identity verification. Post-M1. | RFC-010 |
| LOW | What is the Adaptive Engine? | Runtime hardware decision-making. Post-M1. | RFC-011 |
| LOW | Should State Documents be timestamped by an external authority? | External verification of "this state existed at this time." Post-M1. | RFC-012 |
| LOW | Can State Documents be pruned? | Old states could be archived. Trade-off against audit completeness. Post-M1. | RFC-013 |

## Question Lifecycle

A question moves through these stages:

```
Identified → Discussed → RFC Proposed → RFC Accepted → Decision Recorded → Closed
```

Questions that are not yet ready for an RFC remain in "Identified" status.
