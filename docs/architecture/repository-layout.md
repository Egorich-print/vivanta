# Repository Layout

## Top-level directories

| Directory | Purpose |
|-----------|---------|
| `docs/` | Active architecture documentation, milestones, ADRs, hardware notes |
| `specs/` | Formal specifications — RFCs and schemas |
| `archive/` | Preserved historical artifacts (not authoritative) |
| `vivanta-boot/` | Active boot/kernel code (Rust workspace) |
| `theseus-m1/` | M1 experiment code (to be classified — see below) |

## docs/

```
docs/
├── architecture/       # Current architecture reference
├── milestones/         # Completed milestone reports (M1/M2/M3)
│   ├── M1/A-continuity/
│   ├── M1/B-hardware/
│   ├── M2/
│   └── M3/
├── adr/                # Architecture Decision Records
└── hardware/           # Hardware-specific notes (per platform)
```

**`docs/` is the authoritative source** for active architecture decisions.

## specs/

```
specs/
├── rfc/                # Request For Comments (numbered)
└── schemas/            # Data format specifications
```

RFCs are permanent once numbered. Status (Accepted / Experimental) is in the document header, never in the filename.

Naming: `NNNN-title.md` where `NNNN` is zero-padded to 4 digits.

## archive/

`archive/` contains historical documents from pre-R2 phases. They are preserved for reference but are **not authoritative**. Always check `docs/` and `specs/` for the current version of any concept.

```
archive/
├── README.md
├── milestones/pre-r2/  # R0 review, R1 architecture reduction
├── experiments/        # Experimental code (M1, etc.)
├── goals/              # Archived goal documents
├── mindmaps/           # Archived mind maps
└── research/           # Archived research notes
```

## ADR naming

```
ADR-NNN-title.md
```

Where `NNN` is zero-padded. ADRs are short (1-2 pages) and contain:

```
Status: (Accepted | Proposed | Deprecated)
Context:
Decision:
Consequences:
```

## Milestone directory conventions

Milestones with sub-components use `A-name/`, `B-name/` subdirectories:

```
M1/
├── A-continuity/
│   ├── acceptance.md
│   └── postmortem.md
└── B-hardware/
    └── acceptance.md
```

This keeps multi-phase milestones organized without deep nesting.

## theseus-m1/

The `theseus-m1/` directory contains code from the M1 continuity experiment phase.
Its status is pending audit — see `archive/experiments/m1/` if moved.
