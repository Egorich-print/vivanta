# State Document Schema

**This file has never contained a schema, and none exists anywhere in the
project.** `docs/CHANGELOG.md:27-28` has described this file as a "placeholder"
since 2026-07-11; that is still accurate. Do not cite it as a specification.

## What the format is meant to be

A persistent, hash-chained state document: a public key, a sequence number, the
hash of the previous state, a genesis hash, a hardware inventory, the
environment-state hash, a migration reason, a timestamp, a content hash and a
signature — written as individual files under `/theseus/state/`.

**Defining RFC:** `specs/rfc/0005-state-document-format.md` (RFC-005, *State
Document Format*).

## Why there is no machine-readable schema

- **None has ever been written.** There is no JSON Schema, no CDDL, no
  OpenAPI/YAML document and no CBOR schema file in the repository. The only
  schema-shaped file anywhere under the tree is a target spec in
  `vivanta-boot/archive/`.
- **RFC-005 §8 step 1 was never executed.** The RFC's own "Next Steps" list, at
  `specs/rfc/0005-state-document-format.md:264`, reads "Write the CBOR schema
  specification for State Documents." That step was not done.
- **The only reference implementation contradicts RFC-005's own format
  decision.** RFC-005 §7.1 (`specs/rfc/0005-state-document-format.md:253`)
  fixes the format as **CBOR**, "the fields specified above". The archived host
  experiment at `docs/history/experiments/m1/src/state.rs` does not use CBOR at
  all: its `Cargo.toml` depends on `serde_json` and `hash_state_json` /
  `serialize_state` go through `serde_json::to_string`. So the only code that
  ever touched a `StateDocument` implements the format RFC-005 rules out.
  (RFC-006 already hedged on the neighbouring format: "Manifest format | JSON
  (M2) | Same as State Documents. CBOR deferred." — which is itself at odds with
  RFC-005.)

## Status: blocked, not pending

The subject of this document is behind the project **scope fence**, not behind a
backlog. The fence (`STATUS.md:537-541`) lists storage, Ed25519, BIP-39 and
persistent identity as out of scope until explicitly lifted. A signed, on-disk,
hash-chained state document needs all four. There is no filesystem in the kernel
and no crypto: `execve` serves only the built-in `/init` ELF, and the device UUID
is filled from a monotonic counter (`kernel/src/identity/uuid.rs:20-29`).

Writing the schema now would produce a document describing a system that cannot
exist under the current fence. It is not scheduled work anyone intends to pick
up soon; it is parked.

The archived experiment is also not a safety net: `docs/history/experiments/m1/`
is a `std` host binary outside the workspace, not a member of the 20-member
Cargo workspace, and no CI job can build it.
