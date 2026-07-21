# Architectural Decision Records

This directory indexes ADRs for the Vivanta project (formerly TheseusOS). Engineering-level ADRs (011–023) live in `vivanta-boot/docs/adr/`.

## Active ADRs

| ADR | Title | Status | Location |
|-----|-------|--------|----------|
| ADR-001 | RFC Freeze | Accepted | `vivanta-boot/docs/adr/` |
| ADR-002 | State Versioning | Accepted | `vivanta-boot/docs/adr/` |
| ADR-011 | Phase Transition — Research → Engineering | Accepted (amended 2026-07-19) | `vivanta-boot/docs/adr/` |
| ADR-013 | Privilege Transition Model EL1↔EL0 | Accepted | `vivanta-boot/docs/adr/` |
| ADR-014 | Architectural Boundaries | Accepted | `vivanta-boot/docs/adr/` |
| ADR-015 | Arch Boundary Contracts (`extern "Rust"`) | Accepted | `vivanta-boot/docs/adr/` |
| ADR-017 | Unified Execution Context | Accepted | `vivanta-boot/docs/adr/` |
| ADR-018 | User Entry Transition Model | Proposed | `vivanta-boot/docs/adr/` |
| ADR-019 | User Page Permissions and EL0 Memory Model | Proposed | `vivanta-boot/docs/adr/` |
| ADR-020 | System Runtime Ownership | Accepted | `vivanta-boot/docs/adr/` |
| ADR-021 | BootInfo Escape Prevention | Accepted | `vivanta-boot/docs/adr/` |
| ADR-022 | Minimal Driver Lifecycle Contract | Accepted | `vivanta-boot/docs/adr/` |
| ADR-023 | IdentityState Model | Accepted | `vivanta-boot/docs/adr/` |

## Deprecated ADRs

| ADR | Title | Superseded By |
|-----|-------|---------------|
| ADR-012 | Execution Model — ThreadContext vs ExceptionFrame | ADR-017 |

## ADR Template

Please use the `adr/template.md` file as a guide when creating new ADRs.
