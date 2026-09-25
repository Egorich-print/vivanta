# RFC index — read this first

Vivanta's RFCs ended up in **three** places with overlapping numbers. This file
is the map; nothing here is deleted, because these are design records.

| Location | Status | Contents |
|---|---|---|
| `specs/rfc/` | **Historical drafts (R0–R10 numbering)** | 0001–0007, plus `008`–`010`. Written during the early identity/continuity phase. |
| `vivanta-boot/docs/rfcs/` | **Memory-model contracts (in force)** | RFC-012…RFC-016 — cited by `ADR-011-phase-transition.md` and `current-architecture.md`; they specify what `kernel/src/memory/` actually implements. |
| `vivanta-boot/docs/rfc/` | **Live vision document** | `network-services-vision.md` — linked from `README.md`, `ROADMAP.md`, `evolution-plan.md`. |

## Known collisions (documented, not silently merged)

- **Two RFC-001s.** `specs/rfc/0001-identity-model.md` ("Theseus Identity Model",
  draft) vs `vivanta-boot/docs/rfcs/RFC-001-identity-model.md` ("Persistent
  Device Identity", design intent per ADR-011). Different scope and status; the
  memory-model one is the one ADR-011 refers to.
- **Two boot-protocol RFCs.** `specs/rfc/0003-boot-protocol.md` and
  `specs/rfc/008-boot-protocol.md` cover the same ground under different numbers.
  The boot protocol that shipped is what `vivanta-boot/boot-info/` and
  `kernel/src/lib.rs` implement; treat both drafts as superseded by the code.

## Rules going forward

1. New RFCs that specify implemented behaviour go to
   `vivanta-boot/docs/rfcs/` with the next free `RFC-NNN` number.
2. Vision/exploration documents go to `vivanta-boot/docs/rfc/` (singular).
3. Do not add new files to `specs/rfc/`; that tree is frozen history.

Cleanup record: `docs/audit/2026-09-25-cleanup.md`.
