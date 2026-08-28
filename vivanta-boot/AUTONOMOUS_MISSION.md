# Vivanta Autonomous Mission — G-M10 Hardening to M15

> YOLO режим, аккуратные аудиты, регулярные бэкапы, без небезопасного кода без необходимости.
> Старт: 2026-08-28T00:00Z — backup branch `backup/pre-mission-2026-08-28` пушнут.
> Ветка работы: `main` (каждый этап — документированный коммит → `gh` push)

## Цель миссии
Довести G-M10 "Process Lifecycle + fork/exec" из состояния "собирается, но семантически сломан" (аудит 2026-08-28) до production-grade, затем заложить фундамент для M100 (полноценная ОС).

## Аудит на старте (честный)
* Сборка `aarch64` PASS, но семантика сломана — 7 P0/P1 багов подтверждены 3 суб-агентами (explore).
* `fork` — eager copy ≠ COW, SP_EL1 указывает в чужой стек, PID домен перепутан (TID vs TaskId)
* `waitpid` — рекурсия + `Blocked` без `yield`, очередь без IRQ guard, `wake_waiters(parent_id)` вместо `child_id`
* `kill` — ставит `Exited` вместо `Zombie`, не будит ждущего, поток не терминируется
* `execve` — аллоцирует новый kernel stack и `context_init`, но `eret` берёт старый фрейм на старом SP → UAF, не работает, не подключен к диспетчеру
* `ExceptionFrame` дублируется в `arch-api` и `arch-aarch64`
* `munmap/mprotect` — код верный, тестов нет
* `duplicate_as` хвост `resolve_cow_fault` ставит `LazyAnonymous` вместо `cow_shared`

## План миссии (фазы)
### Phase 1 — Critical Correctness (P0) — текущий
- [ ] 1a: `arch-aarch64/src/context.rs:context_fork` — `child_tc.sp = child_stack_top`
- [ ] 1b: `kernel/src/syscall/process.rs` — PID домен унифицировать на `TaskId` (fork→TaskId, getpid→TaskId)
- [ ] 1c: `ExceptionFrame` унификация (оставить `arch-api`, удалить дубль, проверить `vectors.rs`/`exceptions.rs`)
- [ ] 1d: `waitpid` — убрать рекурсию, добавить `yield_now` под IRQ guard
- [ ] 1e: `kill` — `Zombie` + `wake_waiters(child_id)` + терминировать поток

### Phase 2 — True COW (P0)
- [ ] `kernel/src/vmm/address_space.rs:duplicate_as` → `CoWShared{refcount=2}` с RO обе стороны + `mmu_protect`, без аллокации
- [ ] `resolve_cow_fault` хвост fix (CoW not Lazy) + `unmap_pages`/`unmap_all` рефкаунт
- [ ] Централизованный refcount или кросс-AS обновление

### Phase 3 — Scheduler Hardening
- [ ] IRQ guard для `WAIT_QUEUE`/`RUNQUEUE` + `Blocked→Ready` корректно
- [ ] `wake_waiters(child_task_id)` вместо `parent_id` + `SIGCHLD` as wake only

### Phase 4 — Execve Correctness
- [ ] Откатить сломанный execve к `ENOSYS` stub с честным TODO, либо переписать на `*frame = {elr=entry, sp=new_sp, spsr=0}` на текущем стеке без аллокации нового

### Phase 5 — Рефакторинг
- [ ] Убрать `unsafe` где можно, добавить `// SAFETY:` комментарии, уменьшить дублирование
- [ ] `process_table::children_of` — фильтровать `Exited` tombstone, чистить `parent.children`

### Phase 6 — Верификация
- [ ] Полный `cargo build --workspace --target aarch64-unknown-none` матрица
- [ ] QEMU smoke: `fork/write/waitpid/getppid/kill/munmap/mprotect` (если QEMU доступен)

### Phase 7 — Видение до M100
- [ ] Roadmap файл `Vivanta/docs/adr/ADR-0xx-mission-m100.md`

## Правила
- Каждый этап — атомарный коммит с `Co-Authored` суб-агентов, `git push` через `gh`
- Регулярный `cargo build` после каждой фазы
- Бэкап ветка + тег перед рискованными рефакторами
- Не писать `unsafe` без `// SAFETY:` и проверки
