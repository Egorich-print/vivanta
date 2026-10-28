# Environment Manifest Schema

**This file has never contained a schema, and none exists anywhere in the
project.** `docs/CHANGELOG.md:27-28` has described this file as a "placeholder"
since 2026-07-11; that is still accurate. Do not cite it as a specification.

## What the format is meant to be

A per-environment manifest chained to the state document: a public key, a
sequence number, the previous environment hash, the state hash, a user-data
hash, a configuration hash, a software inventory, a timestamp and a signature.

**Defining RFC:** `specs/rfc/0006-environment-continuity-model.md` (RFC-006,
*Environment Continuity Model*).

## Why there is no machine-readable schema

- **None has ever been written.** There is no JSON Schema, no CDDL, no
  OpenAPI/YAML document and no CBOR schema file in the repository.
- **RFC-005's schema step was never executed**, and no equivalent step was
  carried out for RFC-006 either.
- **The two RFCs do not agree on the wire format.** RFC-005 §7.1
  (`specs/rfc/0005-state-document-format.md:253`) fixes State Documents as
  **CBOR**; RFC-006's format table (`specs/rfc/0006-environment-continuity-model.md:188`)
  says "Manifest format | JSON (M2) | Same as State Documents. CBOR deferred." —
  which contradicts RFC-005's own decision. Any schema written from the RFC text
  alone would have to pick a side in a contradiction the project never resolved.
- **The only reference implementation is JSON.** The archived host experiment at
  `docs/history/experiments/m1/src/environment.rs` uses `serde_json` throughout
  (`hash_manifest_json`, `serialize_manifest`) and its `Cargo.toml` lists
  `serde_json`, not a CBOR crate. The manifest is also stored as pretty-printed
  JSON, not in a per-state file chain.

## Status: blocked, not pending

The subject of this document is behind the project **scope fence**, not behind a
backlog. The fence (`STATUS.md:537-541`) lists storage, Ed25519, BIP-39 and
persistent identity as out of scope until explicitly lifted. A signed,
chained, on-disk environment manifest needs all four. There is no filesystem in
the kernel and no crypto.

This is not scheduled work anyone intends to pick up soon; it is parked behind a
fence that is still up, and the format decision underneath it is unresolved.

The archived experiment is also not a safety net: `docs/history/experiments/m1/`
is a `std` host binary outside the workspace, not a member of the 20-member
Cargo workspace, and no CI job can build it.
