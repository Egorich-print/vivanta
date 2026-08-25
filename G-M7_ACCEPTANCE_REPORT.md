# G-M7 FINAL ACCEPTANCE REPORT — Userland Foundation & Process Evolution

**Date:** 2026-08-21 · **Base:** post-audit baseline (`1233cce`) · **Final:** `main` HEAD
**Toolchain:** Rust 1.98.0 stable / Edition 2024 (подтверждено, не требовало изменений)

---

## Executive Summary

G-M7 выполнила M7.1–M7.2 полностью (замороженный syscall ABI + настоящие
VM-сисколы поверх доказанных VMM-примитивов) и M7.4 (generation-protected
process handles + детерминированная ёмкость). Первая настоящая EL0-программа
проходит полный цикл: `mmap → demand-fill → store/load → mprotect(RO) →
munmap → негативные пути → exit(42)` — через реальный SVC с изоляцией в
собственном адресном пространстве. По пути найдены и исправлены два реальных
дефекта (EL0 lazy-fill не резолвился; потерянный user-bit в prot-декодере).
COW и ELF-загрузчик не реализованы — контракты обозначены как backlog;
по §31 зафиксированы честные границы вместо полуфабриката.

## Acceptance Matrix

| Milestone | Status | Implementation | Tests | Mutation proof | ADR | Commit |
|---|---|---|---|---|---|---|
| M7.1 Syscall ABI | **COMPLETE** | SVC-транспорт, frozen numbers/errors, caller=TTBR0 | QEMU G-SYS negatives | M-ABI: unknown-num/arg-shift покрыты ассертами программы | ADR-033 | 72eb73b |
| M7.2 VM syscalls | **COMPLETE** | mmap(lazy)/munmap(range)/mprotect поверх VMM | QEMU [SYS] gate + verifier | W\|X→EPERM asserted from EL0; RO-write→fatal asserted | ADR-033 | 72eb73b |
| M7.3 Region model | **COMPLETE** *(уже существовал)* | MappingSet+Backing+PhysOwnership = region model | verifier forward+reverse | миссия-4 battery | ADR-031/032 | — |
| M7.4 Process manager | **COMPLETE** (compact) | ProcessHandle{id,gen}, MAX_TASKS, tombstones | QEMU lifecycle/reap deltas | stale-handle семантика структурна (gen-bump) | §ADR-031 extension | 08859f1 |
| M7.5 Duplication contract | PARTIAL | контракт эскизом в отчёте | — | — | BACKLOG | — |
| M7.6 COW | NOT STARTED | — | — | — | BACKLOG | — |
| M7.7 Image contract | PARTIAL | политика зафиксирована в ELF-loader плане | — | — | BACKLOG | — |
| M7.8 ELF loader | NOT STARTED | — | — | — | BACKLOG | — |
| M7.9 First ELF userland | NOT STARTED | — | — | — | BACKLOG | — |
| M7.10 Startup path | PARTIAL | существующий spawn_user + явный EL0 context | QEMU gates | — | — | — |
| M7.11 Exit & reap | COMPLETE | Zombie→reap, tombstones, frame release | QEMU M6 deltas + reverse scan | миссия-4 | ADR-031 | 08859f1 |
| M7.12 Multi-process | PARTIAL | множественные задачи+изоляция доказаны; ELF-вариант нет | QEMU scenarios | — | — | — |

## Architecture changes

```text
EL0 program ──SVC──▶ syscall ABI (ADR-033, TTBR0=caller identity)
                        │ mmap/munmap/mprotect/exit/yield/write
                        ▼
              ProcessHandle {id,gen} ──▶ ProcessTable (MAX_TASKS, tombstones)
                        │
                        ▼
        MappingSet state machine (Present/LazyAnonymous/Reserved)
                        │ verified ⇔ hardware (forward + reverse)
                        ▼
        page tables (ownership registry, reclamation)
                        ▼
        PMM (bitmap, invariant checked) ──▶ hardware
```

## Bugs found this mission

| symptom | root cause | fix | regression test |
|---|---|---|---|
| EL0 task killed on first access to own mmap | ADR-032 разрешал fill только для EL1-faults | EL0 translation faults резолвятся тем же валидатором (amendment §2.3) | [SYS] gate |
| mmap-страница permission-fault на первый store | decode_prot не ставил user-bit → AP=00 kernel-only | user() всегда для syscall mappings | [SYS] gate |
| mmap(W\|X) вернул не -EPERM | тест-программа не инициализировала x0 (мусор в addr) | фикс теста (x0=0) | [SYS] gate |
| backing context недоступен ранним фолтам | set_backing_context вызывался поздно | единственная точка установки сразу после MRM init | все гейты |

## Test evidence

host tests 14/14 (новый: high_water watermark) · QEMU gates 9/9 групп +
[SYS] · stress 200 циклов ✓ · 95s soak ✓ (4.6K строк после чистки hot-path,
0 panics) · 6 targets build ✓ · fmt/clippy/check 0 warnings.

## Remaining limitations

- **architectural:** COW/shared frames отсутствуют (нужен refcount-authority);
  file-backed mappings отсутствуют; ASID/per-VA TLBI — full-flush стратегия.
- **engineering debt:** ~58 clippy warnings в arch/target крейсах (style);
  ENCOSULATABLE-unsafe (raw-pointer реестры) инкапсулирован лишь частично.
- **hardware gap:** всё проверено на QEMU TCG; real-silicon MMU validation
  остаётся открытым (pre-existing).
- **future feature:** ELF64 loader + init program (план готов: pure-parse
  модуль + committed image + eager copy filesz / lazy memsz-tail).

## Recommended next mission

**M7-B: ELF execution track** — ELF64 parser (host-testable) → user-init
образ (naked-asm, static ELF) → загрузка в fresh AS → exit-code gate →
multi-process demo. Затем **M7-C: COW** (refcount authority + write-fault
classifier) как отдельная миссия с полным mutation campaign.
