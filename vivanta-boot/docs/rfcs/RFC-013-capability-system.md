# RFC-013: Capability System

## Status

**Frozen** (per ADR-011).

Design preserved for future revival. Implementation blocked until userspace
isolation and MMU-based enforcement exist.

## Original implementation

`kernel/src/memory/capability.rs` — MemoryCapability with:
- CapabilityId, OwnerId — opaque identifiers
- MemRights — read / write / execute / map / share flags
- MemoryCapability.check() — always returns true (deferred enforcement)
- Integrated with MemoryObject (share() returns ShareHandle with capability)

## Known issues

1. **`check()` returns true unconditionally.** No enforcement mechanism exists.
   The capability struct is decorative.
2. **No derivation model.** Capabilities are created ad-hoc, not derived from
   a root-of-trust. No CNode, no CSlot hierarchy (cf. seL4).
3. **No revocation propagation.** Revoking MemoryObject does not invalidate
   outstanding capabilities.
4. **No IPC transport.** Capabilities cannot be sent between components —
   there are no components.

## Motivation (preserved)

Capability-based security is a core Vivanta differentiator. The model should
follow seL4's proven design (CNode, CSlot, derivation) rather than inventing
a new one. Key requirements:

- Resources are identified by capabilities, not raw handles
- Capabilities are unforgeable (kernel-managed)
- Access rights are embedded in the capability
- Derivation narrows rights (never widens)
- Revocation cascades

## Preconditions for revival

1. Userspace isolation exists (Stage 6)
2. MMU supports per-address-space page tables (Stage 5)
3. A syscall interface exists (Stage 6)
4. At least two resource types need capability gating

## Design questions for revival

1. Should Vivanta adopt seL4's CNode/CDT model directly, or design a
   simplified variant?
2. Should capabilities be memory-backed (like seL4) or handle-backed (like
   Fuchsia)?
3. How does capability revocation interact with MemoryObject revoke?
4. Should capabilities be revoked implicitly on process exit?

## Related RFCs

- RFC-012 (Memory Object) — primary consumer of capability model
- RFC-014 (Hardware Graph) — DeviceObject requires capability-based auth
