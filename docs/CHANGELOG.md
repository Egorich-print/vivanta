# Changelog

## 2026-07-11

### R2 Repository Reorganization

Added:
- `docs/architecture/repository-layout.md` — target directory structure
- `.gitignore` — exclude `target/`, `*.bin`, `.DS_Store`
- `docs/hardware/lavender/` — initial hardware notes for SDM660 target
- `docs/adr/ADR-001-rfc-freeze.md` — placeholder
- `docs/adr/ADR-002-state-versioning.md` — placeholder
- `specs/schemas/` — placeholder for state/environment schemas
- `archive/README.md` — policy for archived documents

Changed (git mv):
- Milestone documents moved under `docs/milestones/`
- RFCs moved under `specs/rfc/`
- ADRs moved under `docs/adr/`
- R0/R1 reviews archived under `archive/milestones/pre-r2/`
- `Goals/`, `MindMap/`, `research/`, `OPEN_QUESTIONS.md` archived
- `theseus-m1/` archived under `archive/experiments/m1/`

Next:
- R2 Reality Lock — Phase 0: Architecture Freeze
