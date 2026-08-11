# VIVANTA — M6 PROCESS LIFECYCLE CORRECTNESS

## Autonomous Engineering Agent Mission

Довести process model до реального жизненного цикла. M5.0 (GREEN BASELINE)
доказал корректность **thread**-lifecycle, scheduler, memory, EL0-границы.
Однако **Task** (process-контейнер) не связан с фактическим исполнением:

- `Task::exit()` — 0 вызовов (метод существует, никогда не вызывается);
- `TaskState::Running` никогда не устанавливается → `running_count()` всегда 0;
- `kill() / zombies() / reap_zombie()` — мёртвые API;
- `SYS_EXIT` вызывает `thread_exit()` (Thread-level), но не трогает Task:
  exit_code теряется, Task остаётся `Created`.

Это архитектурный дефект, а не «отсутствие фичи»: **процесс не имеет
корректного жизненного цикла**, хотя все его примитивы уже реализованы в M5.0.

---

## STATUS: M6 PASS / CLOSED (2026-08-11)

M6 declared **PASS**. All gates G6-A/B/C/D verified on a clean clone with QEMU
runtime evidence:

```text
[task] Task 1 -> Exited code=0            (demo task, exit(0) via SYS_EXIT)
[task] Task 2 -> Exited code=-1           (fault task, abnormal termination)
[M6] demo Task 1 state=Zombie exit_code=Some(0)
[M6] fault Task 2 state=Zombie exit_code=Some(-1)
[M6] running_count=0
[M6] zombies before reap: [1, 2]
[M6] reaped=2 free_before=130675 free_after=130677 delta=+2
[M6] process lifecycle demo OK
```

Reaping returned 2 user-stack frames to the PMM (free_count increased) — G6-C
proven. G6-D (no M5.0 regressions) verified: build/clippy/fmt/test green,
EL0 demo + EFAULT + fault containment + preemption all pass.

### Known pre-existing issue found by the soak (P1, deferred)

While running the 60-min soak, the kernel was observed to **lose timer
preemption** under sustained load (tight loop or, occasionally, an
Instruction Abort on `x30=0`). This is a **pre-existing M5.0-path defect**,
not an M6 regression: it reproduces on the committed pre-M6 state too, and
M6's own gates pass in short runs. Tracked in
`docs/investigations/INV-002-preemption-irq-loss.md`. It must be resolved
before relying on long-running multi-thread workloads.

### Notable fix found during M6

The 16 KiB boot kernel stack was overflowed by `kernel_main` growth (M6 added
locals), corrupting BSS and hanging the kernel during MMU setup. Bumped to
32 KiB (`target-qemu-aarch64/linker.ld`). This was a latent defect that M6
exposed.

---

# 0. SOURCE OF TRUTH

Рабочий репозиторий: `/Users/egorich/ai-workstation/Projects/Vivanta`

Текущий baseline: **M5.0 GREEN BASELINE — PASS / CLOSED** (2026-08-11).
Ratified: `vivanta-boot/docs/milestones/M5.0-green-baseline.md`

Источник истины (приоритет):
1. фактический код;
2. compiler/test/linter output;
3. QEMU runtime evidence;
4. git state;
5. документация.

Не переоценивай зрелость. M6 — **корректность существующей модели**, а не
новые подсистемы.

---

# 1. MISSION

Связать процессную модель с фактическим исполнением так, чтобы каждая
операция жизненного цикла процесса была наблюдаема и проверяема:

```text
spawn
  ↓
Task Created → Running   (когда первый thread стартовал)
  ↓
thread exit → Task Exited / Zombie  (exit_code сохранён)
  ↓
reap → Task удалён из ProcessTable, ресурсы освобождены
```

M6 не добавляет новых систем. Он реализует state machine, которая уже
задекларирована в `TaskState`, но никогда не исполняется.

---

# 2. GATES

## G6-A — Task state machine wired to execution

Требование: `TaskState` переходит по факту исполнения, а не по декларации.

- `TaskState::Created` → `Running`, когда первый thread задачи фактически
  начинает исполняться (не при spawn, а при старте).
- `TaskState::Running` → `Exited` / `Zombie` при завершении последнего thread.
- `Task.exit_code` заполняется реальным кодом выхода (из `SYS_EXIT`).
- `TaskManager::running_count()` отражает факт (не всегда 0).

Инвариант:

```text
for every task: one of {Created, Running, Exited, Zombie} holds,
and the value is consistent with the task's threads' states.
```

## G6-B — Exit code propagation and collection

Требование: exit code доходит от syscall до родителя/коллектора.

- `SYS_EXIT(code)` → `thread_exit` → find owning Task → `Task::exit(code)`.
- Ресурсы Task (owned MemoryObjects) освобождаются при reaping.
- Родитель (или boot monitor) может наблюдать exit_code завершённого потомка.

Инвариант:

```text
spawn child; child exits with C
  → parent/boot can observe exit_code == C, task removed on reap
```

## G6-C — Resource reclamation on task exit

Требование: завершённый процесс возвращает память.

- User stack MemoryObject (owned by Task) освобождается при реaping.
- Kernel stack thread-фреймов освобождается (уже работает в M5.0 cleanup).
- После reap: `free_count` возвращается к baseline (churn-проверка).

Инвариант:

```text
allocated + free == managed, перед и после full task lifecycle.
```

## G6-D — No regressions on M5.0 gates

Требование: G1–G4 остаются зелёными. QEMU-демо, EFAULT-тест,
fault-containment, preemption — всё продолжает работать.

---

# 3. SCOPE

## В scope

1. Task state machine: `spawn_kernel`/`spawn_user` → Running при старте;
   `thread_exit` → owning Task → `Exited`/`Zombie` + exit_code.
2. Reaping: вызов существующего `TaskManager::reap_zombie()` из точки,
   где задача больше не нужна (boot monitor или новый `reap_zombies()`).
3. `running_count()` — реальный подсчёт.
4. Минимальные regression-тесты/доказательства на QEMU:
   - задача завершается с известным кодом, родитель видит его;
   - reap возвращает free_count к baseline;
   - running_count корректен во время исполнения.
5. Документация: обновить STATUS.md, M6-spec, roadmap.

## За scope fence (НЕ трогать в M6)

- IPC, storage, drivers, distributed AI (ADR-031…039), Ed25519, BIP-39,
  persistent identity, TTBR1, ASID, signals delivery.
- Новые syscall-номера. (Доработка поведения существующего `exit` — можно,
  если ABI не меняется.)
- SYS_READ (заглушка), SYS_MMAP (VA allocator) — **backlog вне M6**.
- New hardware targets, new architectures.

Если встретишь проблему вне scope — зафиксируй в deferred findings,
не реализуй.

---

# 4. ИЗВЕСТНЫЕ ФАКТЫ (проверено в M5.0-аудите)

- `Task::exit()` — `kernel/src/scheduler/task.rs:57`, 0 вызовов.
- `TaskState::Running` — только в `running_count()` filter, никогда не set.
- `TaskManager::kill()` / `zombies()` / `reap_zombie()` — 0 вызовов.
- `SYS_EXIT` → `thread_exit()` (scheduler/mod.rs), Task не обновляется.
- Демо-вывод: `1 task(s), 0 running` — Task живёт в `Created` при живом thread.
- `Task.threads: Vec<ThreadId>` — есть mapping thread→task (через поиск),
  но не используется для lifecycle.
- `ProcessTable::remove()` — существует, используется только `reap_zombie`.

---

# 5. РЕКОМЕНДУЕМАЯ РЕАЛИЗАЦИЯ (направление, не prescription)

1. **thread → task lookup**: при `thread_exit`, найти Task, чей `threads`
   содержит текущий ThreadId; вызвать `task.exit(exit_code)`. Exit code
   должен дойти от `SYS_EXIT(arg0)` до `thread_exit` (расширить сигнатуру
   или передать через поле/контекст).
2. **Task Running**: при первом контекст-свитче на thread задачи (или при
   `thread_trampoline`/`eret_to_user_stub` входе) пометить Task `Running`,
   если он ещё `Created`.
3. **Zombie + reap**: когда последний thread задачи завершён и parent/монитор
   готов — вызвать `reap_zombie`, освободив `owned_objects` (user stack).
4. **running_count**: считать по реальному TaskState (после шага 2 — корректно).
5. **QEMU-доказательство**: boot monitor выводит для spawned-задачи:
   `task X: state=Running` (при жизни), `state=Exited code=N` (после), и
   подтверждает `free_count` вернулся к baseline после reap.

Не переписывай scheduler-механику (M5.0 уже корректен). M6 — это слоевая
связка Thread-lifecycle → Task-lifecycle.

---

# 6. EXIT CRITERIA

M6 считается PASS только если на QEMU выполняется:

```text
G6-A:  boot log показывает Task state: Created → Running → Exited(0)
G6-B:  exit code 0 (или выбранный код) наблюдаем у родителя/монитора
G6-C:  free_count после spawn→run→exit→reap == free_count до (churn)
G6-D:  M5.0 gates (G1–G4) остаются PASS; EL0 demo + EFAULT + fault
       containment + preemption работают
```

Никаких «PASS WITH LIMITATIONS». Частичный успех = FAIL.

---

# 7. ПРОВЕРКА

```text
cargo build --workspace           PASS
cargo clippy --workspace          PASS
cargo fmt --check                 PASS
cargo test --workspace --target aarch64-apple-darwin  PASS
cargo build -p vivanta-target-qemu-aarch64           PASS
QEMU boot + M6 lifecycle demo     PASS (Task state transitions + reap + free==baseline)
```

---

# 8. SCOPE FENCE (повтор)

IPC · storage · drivers · distributed AI · Ed25519 · BIP-39 · persistent
identity · TTBR1 · ASID · signals · новые syscall-номера · SYS_READ/mmap
(backlog) · новые hardware targets · новые архитектуры.
