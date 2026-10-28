# RFC index — read this first

Vivanta's RFCs ended up in **three** places with overlapping numbers. This file
is the map; nothing here is deleted, because these are design records. Read the
status column before quoting any of them: most of `specs/rfc/` describes software
that was never built, and one of the two boot RFCs describes a struct the code
does not have.

| Location | Status | Contents |
|---|---|---|
| `specs/rfc/` | **Historical drafts, pre-R2 "Theseus" era. Archived-only — nothing here is implemented by the kernel.** | `0001`, `0001.5`, `0002`–`0007` (zero-padded to four digits), then `008`–`010` (three digits). The digit width changes mid-series, so the two series sort out of order. |
| `vivanta-boot/docs/rfcs/` | **Mixed.** `RFC-012`–`RFC-016` are headed *Frozen* / *Vision document* (per ADR-011) although the memory subsystem they describe was later unfrozen and shipped; `RFC-001` is *Design intent*. The headers are stale, the code is not. | `RFC-001`, `RFC-012`…`RFC-016` — cited by `ADR-011-phase-transition.md` and `current-architecture.md`. |
| `vivanta-boot/docs/rfc/` | **Live vision document** | `network-services-vision.md` — linked from `README.md`, `ROADMAP.md`, `evolution-plan.md`. |

## The numbering situation, exactly

- **Two different documents are both self-labelled RFC-001.**
  `specs/rfc/0001-identity-model.md` is "RFC-001: Theseus Identity Model"
  (draft). `vivanta-boot/docs/rfcs/RFC-001-identity-model.md` is "RFC-001:
  Persistent Device Identity" (design intent per ADR-011). Different scope,
  different status; ADR-011 refers to the second.
- **RFC-011 does not exist.** There is no such file. The only mention of the
  number anywhere in the tree is a post-M1 placeholder row in
  `docs/history/OPEN_QUESTIONS.md:43` ("What is the Adaptive Engine? … RFC-011").
- **The two "boot protocol" RFCs are orthogonal, not duplicates.** Only the
  title collides:
  - `0003-boot-protocol.md` is an **identity-boot state machine** — Stage 0…3,
    three boot modes (GENESIS / NORMAL / RECOVERY), recovery seeds, state
    documents, and a `/sys/theseus/identity/` interface (`:281`).
  - `008-boot-protocol.md` is the **adapter ↔ kernel ABI handoff**: "Vivanta
    Protocol (TBP)", a single `BootInfo` struct every boot path must produce and
    pass to `kernel_main`.
  Different layers, different subjects. Neither is "superseded by the code",
  because the code implements **neither** as written:
  - RFC-008 §1 mandates **9** `BootInfo` fields — `arch`, `memory_map`,
    `boot_source`, `dtb`, `acpi`, `framebuffer`, `cmdline`, `initrd`,
    `cpu_count` (`008-boot-protocol.md:37-47`). The shipped struct has **5**
    (`vivanta-boot/boot-info/src/lib.rs:11-18`): `memory_map`, `mmio_regions`,
    `interrupt_controller`, `cpu_count`, `dtb`. `arch` and `boot_source` are
    gone, `mmio_regions` and `interrupt_controller` are not in the RFC at all,
    and `acpi`/`framebuffer`/`cmdline`/`initrd` were dropped on purpose by
    ADR-015 (`vivanta-boot/docs/adr/ADR-015-arch-boundary-contracts.md:168-170`,
    "No `acpi`, `framebuffer`, `cmdline`, `initrd` (premature)") — **without an
    RFC amendment to 008.** RFC-009 then claims, in its `Replaces` row
    (`009-platform-capabilities.md:7`), to supersede RFC-008 "with richer
    structure"; it is likewise unimplemented.
  - `GENESIS`, `RECOVERY` and `/sys/theseus` appear **zero** times in any live
    `.rs` file. The only hits are in the archived host experiment
    (`docs/history/experiments/m1/src/simulator.rs:24,165,170`).

## What is actually implemented

**Exactly one RFC: RFC-010, the memory resource model** — and even that is still
headed `Status | Draft` (`010-memory-resource-model.md:5`) although it is fully
implemented in `vivanta-boot/kernel/src/memory/` (`resource.rs`, `policy.rs`,
`object.rs`, `manager.rs`, `pmm_backend.rs`, `capability.rs`, `kernel_heap.rs`).
One caveat: `MemoryCapability::check()` is a declared placeholder that always
returns `true` (`kernel/src/memory/capability.rs:74-78`), so the capability half
of the model is nominal.

## What is archived-only

**RFC-001 … RFC-006** (identity model, identity utility model, bootstrap
architecture, boot protocol, recovery seed format, state document format,
environment continuity model). The only code that ever implemented any of them
is the `std` host experiment at `docs/history/experiments/m1/` — crate
`theseus-m1`, edition 2021, a `std` binary that *simulates* a continuity proof
(Ed25519 keys, a BIP-39 word list, JSON state documents, a scripted
GENESIS/NORMAL/RECOVERY walk-through). It is **not a workspace member**
(`vivanta-boot/Cargo.toml` lists 20 members, none of them this), it lives outside
the workspace root, and **no CI job can build it** — `ci.yml` and `release.yml`
only run cargo inside `vivanta-boot/`.

Their subjects — Ed25519 signing, BIP-39 seed phrases, persistent identity, and
state documents living on a filesystem — are explicitly on the project's scope
fence (`STATUS.md:537-541`: storage · Ed25519 · BIP-39 · persistent identity …).
They are not pending work; they are parked behind a fence that is still up.

## The canonical boot contract

Not RFC-008. The real, shipped contract is:

1. `vivanta-boot/boot-info/src/lib.rs:11-18` — the `#[repr(C)]` `BootInfo`, 5 fields.
2. `vivanta-boot/docs/adr/ADR-021-bootinfo-escape-prevention.md` — the
   borrow-and-copy rule: after `SystemState::from_boot_info()` the adapter's
   `BootInfo` must not influence runtime lifetime.
3. `vivanta-boot/docs/architecture/current-architecture.md` §Boot-контракт —
   the prose contract, which states outright that the canonical source is ADR-021
   plus the type itself, **not** RFC-008.

## Known stale headers in `vivanta-boot/docs/rfcs/`

`RFC-012` (Memory Object), `RFC-013` (Capability System), `RFC-015` (Tiered
Memory) and `RFC-016` (Placement Policy) are all headed **"Frozen (per
ADR-011) … Implementation blocked until VMM, page fault handling, and userspace
exist"**. The VMM exists (`kernel/src/vmm/`), the MRM exists
(`kernel/src/memory/`) and EL0 userspace runs. `RFC-014` (Hardware Graph) is
headed "Vision document". **The RFC headers are stale; the code moved on.** Do
not read "Frozen" as a statement about the current tree.

## Rules going forward

1. New RFCs that specify implemented behaviour go to
   `vivanta-boot/docs/rfcs/` with the next free `RFC-NNN` number.
2. Vision/exploration documents go to `vivanta-boot/docs/rfc/` (singular).
3. Do not add new files to `specs/rfc/`; that tree is frozen history.
4. If a decision changes a contract, amend the RFC *or* file an ADR that says
   the RFC is superseded. ADR-015 silently edited RFC-008's field list, which is
   how a 5-field struct ended up under a 9-field specification.

Cleanup record: `docs/audit/2026-09-25-cleanup.md`.
