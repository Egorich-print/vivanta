# Repository Layout

> Verified against the working tree on 2026-09-29 (HEAD `273f9bc`).
> Every path below exists. If you find one that does not, that is a bug in this
> document — fix it rather than the tree.

## Top-level

| Path | Purpose |
|------|---------|
| `docs/` | Project-level documentation: architecture, audits, hardware, milestones |
| `specs/` | Pre-R2 "Theseus" RFCs and schema stubs — **not authoritative** |
| `vivanta-boot/` | All Rust code: the cargo workspace, boot scripts and engineering docs |
| `.github/workflows/` | `ci.yml`, `release.yml` |
| `README.md`, `README.ru.md` | What the project is and how to start |
| `STATUS.md` | Current state (short form) |
| `ROADMAP.md` | Short public milestone summary |
| `LICENSE` | GPLv3 |

Three point-in-time mission reports were moved out of the root into
`docs/history/missions/` (`AUTONOMOUS_MISSION_P0_REPORT.md`,
`AUTONOMOUS_MISSION_REPORT.md`, `G-M7_ACCEPTANCE_REPORT.md`). They are snapshots,
not current state, and two of them contradict `STATUS.md`.

## docs/

```
docs/
├── CHANGELOG.md
├── OS_MATURITY.md
├── REPOSITORY_GUIDE.md
├── manifesto.md
├── session-summary-2026-07-17.md
├── architecture/       # current-architecture reference + roadmap (4 files)
│   ├── arm-exception-state.md
│   ├── assumption-register.md
│   ├── master-roadmap.md
│   └── repository-layout.md
├── audit/              # audit results — the open-defect backlog lives here
│   ├── README.md
│   ├── 2026-09-24-global-audit.md
│   └── 2026-09-25-cleanup.md
├── archive/            # superseded research track (see below)
│   └── research-track/
├── experiments/        # active, numbered experiments (EXP-001, EXP-002)
├── hardware/           # per-board notes: aibox-1684x, lavender, rk3568, rpi3b, whyred
├── history/            # FROZEN pre-R2 archive — do not edit
├── milestones/         # completed milestone reports (M1–M4, R2)
├── research/           # exploratory notes
└── tooling/            # CI/CD documentation (ci-cd.md)
```

**`docs/audit/` is the source of truth for open defects.**
`docs/audit/2026-09-24-global-audit.md` is the current correctness and
silicon-blocker backlog. When documentation and code disagree, that file is
where the disagreement is recorded.

**`docs/history/` is frozen.** It is a 95-file, 7.4 MiB snapshot of the project
under its earlier names ("Theseus OS", "Vorkuta OS"). Do not edit it, do not
link into it as if current, and do not resurrect its paths. Its
`README.md` says the contents are "not authoritative", but only one of its
`.md` files actually carries that banner — so do not rely on the files
self-declaring.

**`docs/archive/` is not `docs/history/`.** It holds one superseded research
track (`docs/archive/research-track/`): a digest, evolution notes, a
distributed-infrastructure suite of five long documents, and ten unratified
ADR drafts. It is a graveyard, not an archive of record.

## specs/

```
specs/
├── rfc/                # 11 pre-R2 "Theseus" RFCs
└── schemas/            # 2 files, neither has ever held a schema
```

`specs/` predates the R2 reorganisation and **is not authoritative**. The two
RFCs that describe boot state (`specs/rfc/0003-boot-protocol.md`,
`specs/rfc/008-boot-protocol.md`) are orthogonal to each other, and the shipped
`BootInfo` matches neither. The canonical boot contract is ADR-021 plus
`vivanta-boot/docs/architecture/current-architecture.md`.

Naming: `NNNN-title.md`, zero-padded. Status (Accepted / Proposed /
Superseded) lives in the document header, never in the filename.

## vivanta-boot/

### Workspace members — 20

`vivanta-boot/Cargo.toml:5-34` defines exactly 20 members:

| Group | Crates |
|-------|--------|
| Shared libraries | `vm`, `exec`, `boot-info`, `boot_common`, `arch-api`, `arch-aarch64`, `arch-armv7a` |
| Kernel | `kernel` |
| Platform | `platform-qemu`, `platform-rk3568`, `platform-sdm660`, `platform-allwinner-h616`, `platform-amlogic`, `platform-rpi3b` |
| Target binaries | `target-qemu-aarch64`, `target-rk3568`, `target-x96q`, `target-lavender`, `target-rpi3b-plus`, `target-suma-q5` |

Dependency graph caveats, verified:

- `vivanta-boot/user-init/` is **not** a workspace member.
  `vivanta-boot/user-init/Cargo.toml:8` opens its
  own empty `[workspace]`. It is built by `vivanta-boot/tools/build-user-init.sh`
  and embedded into the kernel at compile time by `include_bytes!` at
  `vivanta-boot/kernel/src/syscall/process.rs:545`. Because `*.elf` is
  gitignored (`.gitignore:13`), **a fresh clone cannot build `vivanta-kernel`
  until that script has been run** — every entry point handles this
  (`vivanta-boot/build.sh:19-22`, `vivanta-boot/tools/qemu-gates.sh:52-53`,
  `.github/workflows/ci.yml:60-64`).
- `platform-rk3568` is a member but is **linked by no target**. It appears only
  in the members list and its own manifest. It is currently a dead crate.
- `target-rk3568` declares **no** `vivanta-kernel` dependency at all — it is an
  EL2-stage UART/FDT diagnostic that never reaches `kernel_main`.
- `target-lavender` declares `vivanta-kernel` but never calls it; it prints
  five lines and spins (`vivanta-boot/target-lavender/src/main.rs:71-87`).

### Not built — 5 directories

These directories have a `Cargo.toml` but are outside the workspace, so
`cargo` refuses to touch them. Running any cargo command inside one fails
with:

```
error: current package believes it's in a workspace when it's not:
current:   .../vivanta-boot/<dir>/Cargo.toml
workspace: .../vivanta-boot/Cargo.toml
```

| Path | What it is |
|------|-----------|
| `vivanta-boot/kernel-memory-frozen/` | Pre-R2 memory model kept as an archival record |
| `vivanta-boot/user/hello/` | Superseded Rust userspace program |
| `vivanta-boot/user/libc/` | Superseded libc skeleton |
| `vivanta-boot/archive/boot_legacy/` | Legacy boot adapters (aarch32 qemu_virt, aarch64 qemu_kernel / lavender, platforms/rk3568) |
| `vivanta-boot/tests/` | One `minimal_test.rs` that declares itself "not a real test file" |

They are preserved deliberately. `vivanta-boot/docs/adr/ADR-011-phase-transition.md:162`
("Frozen systems (archived to RFC, not deleted)") is the decision that keeps
them. Note that ADR-011's freeze list is itself partly outdated: the memory
model it froze has since been unfrozen and integrated into
`vivanta-boot/kernel/src/memory/`. The directories above are what remain.

**Do not "fix" this by adding them to the workspace.** They are reference
material, and their code no longer matches the kernel's contracts.

### Scripts

| Path | Role |
|------|------|
| `vivanta-boot/build.sh` | default build entry point |
| `vivanta-boot/flash_rk3568.py` | RK3568 flashing (superseded timing — see the audit) |
| `vivanta-boot/tools/build-user-init.sh` | builds and stages `user-init.elf` |
| `vivanta-boot/tools/ci-local.sh` | reproduce the CI pipeline locally |
| `vivanta-boot/tools/make-rpi3b-image.sh` | builds the RPi3B+ SD image |
| `vivanta-boot/tools/qemu-gates.sh` | **the gate** — 18 required serial markers |
| `vivanta-boot/tools/rust-objcopy.sh` | ELF post-processing |
| `vivanta-boot/tools/soak_test.sh` | long-running QEMU soak |

### vivanta-boot/docs/ — engineering documentation

This is a **second documentation tree**, distinct from the project-level
`docs/`. It is where ADRs live.

| Path | Contents |
|------|----------|
| `vivanta-boot/docs/adr/` | **18 ADRs** — the canonical decision record |
| `vivanta-boot/docs/architecture/` | engineering architecture (12 files + `milestones/`) |
| `vivanta-boot/docs/investigations/` | 5 written investigations (INV-001…003 + 2) |
| `vivanta-boot/docs/milestones/` | 3 milestone specs (M5.0, M6, G-M10) |
| `vivanta-boot/docs/review/` | 3 review artefacts, correctly marked historical |
| `vivanta-boot/docs/rfc/`, `vivanta-boot/docs/rfcs/` | 1 vision doc + 6 RFCs |
| `vivanta-boot/docs/*.md` | 5 standalone notes at the top level |

> **ADR number collision.** `docs/archive/research-track/` contains its own
> `ADR-031`…`ADR-039` with entirely different titles (node taxonomy, workload
> model, accelerator interface, …) plus an `ADR-018` root copy. They are
> unrelated to the ratified ADRs of the same numbers in
> `vivanta-boot/docs/adr/`. Never cite an ADR number without its directory.

## ADR naming and location

ADRs live in **`vivanta-boot/docs/adr/`**, not in `docs/adr/`.

```
ADR-NNN-kebab-case-title.md
```

Existing set: ADR-011…015, 017…023, 030…035 — 18 files. **ADR-016, ADR-024 and
ADR-025 do not exist**; any document that cites them is wrong. ADR-012 is
marked superseded by ADR-017. ADR-018, ADR-019 and ADR-035 are `Proposed`.
Several older files (ADR-014, 015, 020–023, 030) carry no `## Status` heading
at all, so read the body rather than assuming.

Template:

```
Status: (Accepted | Proposed | Superseded)
Context:
Decision:
Consequences:
```

## Milestone directory conventions

Milestones with sub-components use `A-name/`, `B-name/` subdirectories:

```
docs/milestones/M1/
├── A-continuity/
│   ├── acceptance.md
│   └── postmortem.md
└── B-hardware/
    └── acceptance.md
```

Later milestones use a flat `M{n}/acceptance.md`
(`docs/milestones/M2/acceptance.md` … `docs/milestones/M4/acceptance.md`),
and the pre-R2 `docs/milestones/R2/reality-lock.md` uses the flat form too.

## Repository hygiene

- 443 tracked files; `.git` is 4.9 MB. (Measured 2026-09-29; the count moves
  whenever work lands — re-measure with `git ls-files | wc -l`.)
- **The single largest tracked file is a 5.6 MB minified JavaScript bundle**,
  `docs/history/mindmaps/MindMap/Создание собственной ОС_files/index-86c718b2.js`
  — a FlowUs mindmap export, a web page saved whole. `docs/history/mindmaps/`
  is 7.1 MiB of the 7.4 MiB archive — ~96% of its bytes — and ~76% of all
  tracked bytes in the repository. The bundle alone is ~60% of the whole repo.
- This is a **candidate for removal, not a fine state of affairs.** The content
  is a duplicate of the mindmap's own HTML rendering; the JS bundle carries no
  information the `.html` and `.png` beside it do not. If you touch
  `docs/history/mindmaps/`, do not preserve it by default.
- `docs/audit/` holds two real audits and a `README.md` index. The third audit-
  shaped file, `docs/history/audit-prompts/Independent_Architecture_Audit.prompt-template.md`,
  is a prompt template for external LLMs with no answers filled in. It now
  carries `.prompt-template.md` and lives under `docs/history/`, and it is easy
  to mistake for a third audit if you meet the old path in an older commit.

## Note on this document's history

An earlier version of this file described `archive/`, `archive/milestones/pre-r2/`,
`archive/experiments/`, `archive/goals/`, `archive/mindmaps/`,
`archive/research/`, `theseus-m1/` and `docs/adr/` — none of which exist. That
version was written for the pre-R2 tree and was not maintained through the R2
reorganisation (`6a62c7b`, 2026-07-11) or the project rename (`940832b`,
2026-07-19).

Commit `65eff7f` (2026-09-25) fixed a file with this same name at a **different
path** — `vivanta-boot/docs/architecture/repository-layout.md`. That one is
current. This file was untouched between 2026-07-19 and now.
