# RFC-001: Persistent Device Identity

## Status

**Design intent** (per ADR-011).

Not frozen. Preserved as active architectural goal. Implementation blocked
until secure boot, persistent storage, and userspace isolation exist.

## Motivation

Vivanta derives a persistent cryptographic identity from hardware properties:

- BIP-39 seed derived from device-unique serials
- Ed25519 keypair generated at first boot
- State document preserved across reboots (environment continuity)
- Identity used for: device attestation, secure boot verification,
  capability signing, network authentication

The M1 experiment proved this concept works on real hardware. It is Vivanta's
strongest architectural differentiator from "yet another Unix-like kernel."

## Status per ADR-011

```
Stage    │ Identity component
─────────┼──────────────────────────────────────
Stage 1-5│ No identity work
Stage 6  │ Userspace provides process isolation
Stage 7  │ Storage driver → persistent state
Stage 8+ │ Secure boot infrastructure
Stage 9+ │ Identity revival from this RFC
```

## Key references

- M1 experiment artifacts (archived during R2 reorganization)
- Ed25519 key generation validated on RK3568-class hardware
- State document serialisation proven in M1

## Related RFCs

- RFC-013 (Capability System) — identity is the root of capability derivation
- RFC-014 (Hardware Graph) — device identity may incorporate hardware identity
