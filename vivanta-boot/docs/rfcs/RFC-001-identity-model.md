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

The M1 continuity experiment demonstrated the concept, but **not on hardware and
not in Vivanta**: `docs/history/experiments/m1/` is a `std` host binary
(`theseus-m1`, outside the 20-member workspace, no CI job builds it). No Vivanta
kernel has ever booted on a physical board. It is Vivanta's intended
architectural differentiator from "yet another Unix-like kernel."

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
