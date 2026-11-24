# Mission reports (archive)

Point-in-time reports from Vivanta's autonomous-mission runs. They were kept at
the **repository root**, where their filenames — `AUTONOMOUS_MISSION_REPORT.md`,
`G-M7_ACCEPTANCE_REPORT.md` — read like live project status. They are not.

Each file here now carries a `RETIRED` banner at the top naming its own date and
listing the specific claims that are stale. **The bodies are preserved
verbatim**; nothing was rewritten. They are valuable records of what the project
knew and believed at the moment each mission closed, including the reasoning
behind decisions that were later reversed.

## What is here

| Report | Dateline in file | Mission | Superseded by |
|---|---|---|---|
| [`AUTONOMOUS_MISSION_REPORT.md`](AUTONOMOUS_MISSION_REPORT.md) | 2026-08-21 | M6.0 user virtual memory & fault-driven mapping (mission 4) | mmap/munmap/mprotect (`72eb73b`), COW (`e883af7`/`661e22e`), ELF loader (`f8039b0`), reverse verifier (`69f556b`) |
| [`G-M7_ACCEPTANCE_REPORT.md`](G-M7_ACCEPTANCE_REPORT.md) | 2026-08-21 | G-M7 userland foundation & process evolution | G-M8 ELF loader (`f8039b0`), G-M9 COW (`e883af7`/`661e22e`) |
| [`AUTONOMOUS_MISSION_P0_REPORT.md`](AUTONOMOUS_MISSION_P0_REPORT.md) | 2026-09-09 | P0 corrective mission after the 2026-09-08 audit | the 2026-09-24 global audit and the 2026-09-25 kernel fixes |

Note that all three datelines disagree with git: the G-M7 report is dated
**2026-08-21** but all three of its cited commits are **2026-08-25**, and the P0
report is dated **2026-09-09** but all four of its cited commits are
**2026-09-15**. The bodies were written or amended after their stated dates. The
banners record this.

## Why they were moved out of the repository root

A report at the root of a repository is found first. Three of them sitting at the
root meant the first thing a reader — or an automated agent — encountered was a
set of **superseded** status claims presented in the present tense:

- the P0 report's "19/19 QEMU gates PASS" (the gate now requires exactly **18**
  markers, and the CI job is **advisory**: `continue-on-error: true`);
- the G-M7 report's "M7.6 COW / M7.8 ELF loader — NOT STARTED" (both shipped on
  2026-08-26/27);
- the M6.0 report's `MAX_ADDRESS_SPACES=8` (now **16**) and a backlog list whose
  top three items had all shipped within a week.

Filed next to `STATUS.md` and `ROADMAP.md` at the root, they actively contradicted
them. Moved into `docs/history/`, they are what they always were: evidence of
how the project worked.

## Authority

**Nothing in this directory is authoritative.** `docs/history/README.md` already
declares the wider `docs/history/` tree non-authoritative; this directory is
governed by that statement.

The current state of Vivanta is:

- [`STATUS.md`](../../../STATUS.md) — what is built, running and open
- [`ROADMAP.md`](../../../ROADMAP.md) — what is planned
- [`docs/audit/2026-09-24-global-audit.md`](../../audit/2026-09-24-global-audit.md) — the open correctness backlog
- [`docs/audit/2026-09-25-cleanup.md`](../../audit/2026-09-25-cleanup.md) — what the 2026-09-25 cleanup removed
- [`vivanta-boot/tools/qemu-gates.sh`](../../../vivanta-boot/tools/qemu-gates.sh) — the 18 required serial markers, i.e. what "the gates pass" actually means

If a report here disagrees with one of those, the report is wrong.

## A note on names in this tree

`docs/history/` is a frozen pre-R2 archive and still uses the project's former
names — **Theseus OS** and **Vorkuta OS**. That is expected for a historical
archive: the rename to *Vivanta* happened on 2026-07-19 (`940832b`). Only 1 of
its 63 markdown files carries a non-authoritative banner, so treat the whole
tree accordingly.
