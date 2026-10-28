# 30 — Процессы, планировщик, гейты: lifecycle-бэклог

Миссия: кластер «процесс/планировщик/teardown» + целостность гейтов.
Область: `vivanta-boot/kernel/src/{syscall*,scheduler,signal,vmm}`, гейты в
`vivanta-boot/kernel/src/lib.rs`, маркеры в `vivanta-boot/tools/qemu-gates.sh`,
EL0-блобы в `vivanta-boot/arch-aarch64/src/user.rs`, `vivanta-boot/user-init/`.

Проверка выполнена по коду на 2026-09-29 (HEAD `273f9bc`). Каждое утверждение
снабжено `path:line`. Метки: **CONFIRMED** (прочитал код, дефект/факт
непосредственно в указанной строке), **INFERRED** (вывод из нескольких прочитанных
мест, помечен явно), **UNCERTAIN** (определить не удалось — так и написано).

Эффекты оцениваются только в относительных единицах: XS / S / M / L / XL.

**Что проверено и оказалось пустым (чтобы не искали повторно).**
`vivanta-boot/kernel/src/state/{mod,hardware}.rs` (86 + 55 строк) прочитаны
целиком: `SystemState` содержит только `HardwareState`, `MemoryResourceManager`
и `IdentityState` (`kernel/src/state/mod.rs:13`, `:43`, `:59-83`). Ни типа
процесса, ни состояния планировщика, ни AS-реестра там нет — этот файл в
кластер не входит, и ядро процесса целиком в `scheduler/` + `syscall/`.
Аналогично `kernel/src/scheduler/task_manager.rs` (146 строк) — это тонкая
обёртка над глобальной `ProcessTable`, а не отдельный менеджер; собственного
состояния не держит, поэтому новый `teardown`-модуль логично положить рядом с
ним, а не расширять его.

---

## 1. Таблица верифицированных дефектов

Аудит `docs/audit/2026-09-24-global-audit.md` датирован 2026-09-24; часть
пунктов закрыта коммитами после него. Ниже — фактическое состояние.

| ID | Статус | Evidence | Реальная severity | Реальный blast radius |
|---|---|---|---|---|
| **P0-1** `waitpid` не освобождает ресурсы | **CONFIRMED**, без частичных фиксов | `kernel/src/syscall/process.rs:285` — `let _ = process_table().remove(child_id);` и всё. `ProcessTable::remove` (`scheduler/process_table.rs:101-121`) оставляет tombstone, который сохраняет только `threads.first()` (`:111-115`) и `address_space` (`:115`), но никак не трогает реестр AS | P0 | Реальный радиус шире «теряется один AS»: (а) AS остаётся в `ADDRESS_SPACES` навсегда → через 7 пользовательских fork'ов пул из 16 слотов исчерпан (расчёт в §1.1) → паника в `register_child` (`vmm/address_space.rs:1087`) прямо из EL0; (б) потоки ребёнка остаются в runqueue (ядро освободит их стеки только при *следующем* `thread_exit` — `scheduler/mod.rs:589-619`); (в) COW-фреймы ребёнка: refcount уменьшается только при снятии маппинга, то есть никогда → утечка физпамяти; (г) `sys_kill` для удалённой цели идёт мимо `thread_exit` (см. P1-kill) → те же потери без всякого `waitpid` |
| **P0-5** `execve` не транзакционен | **CONFIRMED** | `syscall/process.rs:613-615` (`unmap_all`) предшествует `:621-628` (`load_elf`, может вернуть `Err`) и `:647-652` (`reserve_at` стека, тоже `Err`) | P0 | Любой отказ возвращает управление в уже размапленный образ. `unmap_all` снимает и `mmu_unmap` (`vmm/address_space.rs:864`), и shadow → следующий fetch по старому `ELR_EL1` даёт instruction abort с lower EL → `el0_sync_handler` (`arch-aarch64/src/user.rs:516-560`) → контейнмент → `user_fault_terminate` → `thread_exit(-1)`. Пользователь видит не errno, а смерть задачи с кодом −1. Срабатывает на любом OOM/переполнении `MappingSet` (ёмкость 64, `vmm/mapping.rs:147`) |
| **P0-8** исчерпание пула AS — паника на пользовательском пути | **CONFIRMED** | `vmm/address_space.rs:1087` (`panic!("address space registry full")` в `register_child`), `:1102` (то же в `register`); `syscall/process.rs:145` вызывает `register_child` без проверки. Плюс `scheduler/mod.rs:267` — `handle.expect("process table full")` в `create_task_for_thread` | P0 | Паника, достижимая из `svc` в userspace. Не «теоретический» дефект: пул уже занят на 9 из 16 слотов самими boot-гейтами (§1.1) |
| **P0 (silicon) 3** cache-maintenance только для страницы входа | **CONFIRMED** | `syscall/process.rs:637-639` — `mmu_flush_icache_range(entry & !0xFFF, 4096)`. Путь возврата — SVC-эпилог `arch-aarch64/src/vectors.rs:166-190`, где `eret` идёт **без** `ic iallu` (сравните `arch-aarch64/src/user.rs:50-55`, где `ic iallu` есть) | P1 на QEMU / P0 на кремнии | Сегодня **не наблюдаемо**: реальный `user-init.elf` имеет один `PT_LOAD` `flags=0x5 (R|X)`, `vaddr=0x1000000`, `filesz=0xf8` (проверено разбором заголовка) — сегмент меньше страницы. Воспроизводится только на образе с `.text > 4 KiB` на настоящем CPU. Значит гейт может проверить только **покрытие аргумента** вызова, но не сам эффект |
| **P1** поток публикуется `Ready` до появления `Task` | **ALREADY FIXED** (коммит `d083f8e`) | `scheduler/mod.rs:235` (`ThreadState::Created` в `create_thread_with_context`), `:300` (`create_user_thread`), `:340` (`create_kernel_thread`) — `thread_set_state(..., Ready)` убран; добавлен `publish_thread` (`:276-278`), вызываемый в `task_manager.rs:78`, `:112` и `syscall/process.rs:179` | — (закрыт) | Фикс полный для всех трёх путей создания. **Но** фикс неполон в смежном месте: при переполнении таблицы `spawn_kernel` (`task_manager.rs:100-103`) снимает поток, но **не возвращает его kernel-стек** в PMM — `spawn_user` это делает (`:62-68`). Утечка 64 KiB на каждый неудачный `spawn_kernel` |
| **P1** `TaskState::Created → Running` не бывает для user-потоков | **CONFIRMED** | Переход живёт только в `thread_trampoline` (`scheduler/mod.rs:459-466`), а он — EL1-трамплин kernel-потоков. User-поток входит в EL0 через `eret_to_user_stub` (`arch-aarch64/src/user.rs:19-56`) — чистый asm, Rust-кода нет. `Task::new` ставит `Created` (`scheduler/task.rs:59`) и он больше не меняется | P2 (не P1) | Реальный ущерб мал: единственные потребители — две печати `running_count=` (`kernel/src/lib.rs:1030-1034` и `:1969-1972`), и **ни одна из них не является маркером** `qemu-gates.sh`. G4-гейт использует `running_thread_count()` (`scheduler/mod.rs:131-135`) — это счётчик по `ThreadState`, он корректен. Настоящая цена не в счётчике, а в том, что модель lifecycle задачи опробована наполовину, а новый teardown-путь будет опираться на неё |
| **P1** синтетический `[FORK]`-гейт подделывает `CURRENT_THREAD` и оставляет «готовые» потоки с пустыми AS | **CONFIRMED** | `kernel/src/lib.rs:1423-1464` — `set_current_thread_id` без IRQ-защиты (сравните `scheduler/mod.rs:120-122`, где документировано требование «single-core IRQ-guarded»). Потоки созданы `spawn_user` (`:1369-1378`, `:1386-1395`) с entry `0x5E00_0000` в AS, собранных с `user_pages = None` (`:1366`, `:1384`) → кода по этому адресу нет; `spawn_user` публикует их `Ready` (`task_manager.rs:78`) | P1 | Три независимых искажения: (а) тик в окне `1423…1464` может запланировать «процесс» с пустым AS → instruction abort → `thread_exit(-1)` прямо посреди гейта; (б) `sys_getpid`/`sys_getppid`/`sys_waitpid` проверяются не через SVC, а прямым вызовом с подменённой идентичностью; (в) выход ребёнка **не проверяется вообще** — `Task::exit(7)` вызывается напрямую (`:1442-1445`), `thread_exit` не проходит. Гейт не маскирует P0-1, но и не проверяет ничего из lifecycle |
| **P1** `[FORK-EL0] child teardown PASS` при невыполненной ветке + родитель не проверяет код выхода/`status` | **PARTIALLY FIXED** | Фикс `602fe7b` заменил `if let Some(...)` на `let ... else { panic!(...) }` — `kernel/src/lib.rs:476-478`. **Что не исправлено:** (1) родительский блоб передаёт `status = NULL` (`arch-aarch64/src/user.rs:366`) и проверяет только `wait_ret == child_pid` (`:370-373`) — код выхода ребёнка 7 и `status` не проверяются никем; (2) сам teardown делает **гейт**, а не ядро (`:479-488`), т.е. маркер маскирует P0-1; (3) ребёнок ищется как tombstone с `state == Exited` (`:471`) — то есть маркер доказывает лишь, что `waitpid` снял запись из таблицы | P1 | Метка в `qemu-gates.sh:33` зелёная при полностью сломанном освобождении ресурсов. Это худший вид вранья: гейт, который специально существует, чтобы доказывать освобождение, доказывает противоположное |
| **P1** `kill` без проверки полномочий | **CONFIRMED** | `syscall/process.rs:323-405` — ни одного сравнения с идентичностью/владением. В `scheduler/task.rs:35-49` нет ни `uid`, ни capability; `memory::capability::OwnerId` (`kernel/src/memory/capability.rs:4`) существует, но к процессам не подключён | P1 (сейчас) / P0 (когда появится userland) | Сейчас — косметика (в системе один пользователь). Как только появится настоящий userland, это тривиальный привилегированный обход всех защит: `kill(1, 9)` из любого EL0-процесса |
| **P1** `execve` игнорирует `argv`/`envp`, начальный стек не строится | **CONFIRMED** | `syscall/process.rs:582-583` (`_argv`/`_envp`), `:644-653` — стек это одна страница `0x3FFF_F000` без guard, без строк, без auxv; `:667` — `(*frame).x = [0u64; 31]`, комментарий `:668` признаёт «x0 would be argc if we set up argv; for now 0» | P1 | Ни один реальный ELF-образ не может быть запущен: нет `argc`/`argv` → невозможен ни один инструмент (аргументы, окружение), невозможен `__libc_start_main`-шаблон. Это главный функциональный блокер userland, а не «недостаток полировки» |
| **P1** внутренние root-фреймы `sys_fork` не возвращаются в PMM | **CONFIRMED** | `syscall/process.rs:81-87` — `stack_alloc.alloc_frame()` из PMM; в успешном пути фрейм не освобождается нигде. `vmm::unregister` (`vmm/address_space.rs:1113-1140`) комментирует `:1109-1111`: «root frames are boot-allocated, untracked, and therefore never reclaimed» — для fork-созданных root'ов это **неверно**, они взяты из PMM | P1 | 4 KiB на каждый fork, навсегда. Второй, независимый ущерб: `unregister` не может их вернуть даже при правильном teardown → нужен `AddressSpace.root_owned: Option<PhysFrame>` |
| **Signals** `pending` write-only, `flags` не интерпретируются, живёт только SIGKILL | **CONFIRMED** | `kernel/src/signal.rs:134-140` — явный scope fence; читателей `pending` нет нигде, кроме гейта `kernel/src/lib.rs:270`. `SigAction.flags` (`signal.rs:55`) пишется (`:508`) и печатается (`:510`), но не читается нигде. `sys_kill` действует только в ветке `Signal::Kill` (`syscall/process.rs:353-394`); остальное — «just queue» (`:396-404`). `SYS_RT_SIGRETURN=15` → `ENOSYS` (`kernel/src/syscall.rs:39`, `:134`); `SYS_RT_SIGPROCMASK=14` → `ENOSYS` (`:133`) | P1 | Пользовательский `SIGCHLD` не приходит никогда → event-driven userland невозможен; polling вместо сигналов. Это блокер для `user-init` как init-процесса |

### 1.1. Новые дефекты этого кластера, которых нет в аудите

| ID | Статус | Evidence | Severity | Blast radius |
|---|---|---|---|---|
| **N-1** `pending: Option<Signal>` **затирает** второй сигнал | **CONFIRMED** | `kernel/src/signal.rs:84-93` — `self.pending = Some(sig)` без проверки занятости. `kill(pid, SIGCHLD)` затем `kill(pid, SIGTERM)` → SIGCHLD потерян | P1 | Поле `pending` неверно не только «нет доставки», но и «не хранит очередь». Любая доставка поверх `Option<Signal>` унаследует потерю. Фикс: `pending: u64` (битмаска) — ровно POSIX-семантика стандартных сигналов (слияние) |
| **N-2** `sys_execve` затирает x19–x28 — нарушение callee-saved ABI | **CONFIRMED**, **не в аудите** | `syscall/process.rs:667` обнуляет все 31 GPR. ADR-033:36 фиксирует «`x1`–`x18` call-clobbered», что подразумевает сохранность `x19`–`x28`. Живой контрпример: fork-блоб держит pid ребёнка в `x19` (`arch-aarch64/src/user.rs:364`), vmsys-блоб — базу mmap в `x19` (`user.rs:243`) | P1 | Процесс, сделавший `fork()` или `mmap()` и затем `execve()`, теряет x19–x28 на выходе из execve. `execve` не обязан сохранять x0–x18 (там возвращаемое значение), но обязан сохранять x19–x28. Ломает любой будущий `_start` на Rust/asm, который передаёт аргументы через callee-saved |
| **N-3** блокирующий `waitpid` уходит в livelock | **CONFIRMED**, **не в аудите** | `syscall/process.rs:315-317` — `wait_for_child(wait_id); yield_now();`. `yield_now` при отсутствии Ready-потока просто `return` (`scheduler/mod.rs:363-368`), не переключаясь. Поток при этом `Blocked` и повторно блокируется в следующей итерации цикла `waitpid` (`:309-318`) | P1 | Если блокирующийся `waitpid` — последний не-idle Ready-поток (например, boot-поток вызывает его, или все дети уже мертвы, а родитель — единственный), пользовательский код **намертво крутится** в одном syscall, IRQ включены, переключения нет. Сегодня не видно, потому что `boot` всегда `Ready`, когда работает user-поток. Нужен либо `Sleeping`-состояние с явным пробуждением, либо panic-gate «waitpid без runnable-партнёра — невозможное состояние» |
| **N-4** idle-поток создан с `entry = 0` — вооруженная ловушка | **CONFIRMED** (чтение), **INFERRED** (достижимость) | `scheduler/mod.rs:653-658` — `context_init(idle_top, idle_bottom, 0, 0, ExecutionLevel::Kernel)`; 4-й аргумент — `entry` (`arch-api/src/context.rs:63-69`), значит `x30 = 0`. При этом `thread_exit` ищет следующий поток с `exclude_idle = false` (`scheduler/mod.rs:542`), то есть **idle разрешён** | P1 | Если idle когда-либо станет единственным кандидатом, `context_switch` выполнит `ret` по `x30 = 0` → немедленный EL1 abort. Сегодня недостижимо только потому, что boot-поток всегда `Ready` (его ставит `yield_now` перед переключением), и Normal-приоритет выигрывает у Idle в первом же цикле `find_next_ready`. Это ровно тот класс дефекта, что прячет гейт «готовые потоки с пустыми адресными пространствами»: он не срабатывает не потому, что корректен, а потому, что порядок обхода маскирует |
| **N-5** `mprotect`/`munmap` возвращают не тот errno, что предписан ADR-033 | **CONFIRMED**, **не в аудите** | ADR-033:95 — MUNMAP «range must intersect a mapping else `-ENOMEM`»; ADR-033:101 — MPROTECT «range must be covered by mappings else `-ENOMEM`». Код: `syscall/process.rs:704-707` и `:733-736` отдают `EFAULT`. Частично описанное аудитом несоответствие (`W\|X → -EINVAL` вместо `-EPERM`, `:722-724`) — только половина: `mmap` это делает правильно (`kernel/src/syscall.rs:180-182`) | P2 | Второй, не описанный в аудите, класс расхождения с замороженным контрактом. `kill` — третий: `syscall/process.rs:336-337` возвращает `EINVAL` с комментарием `// ESRCH` |
| **N-6** boot-гейты сами занимают 9 из 16 слотов AS | **CONFIRMED** | Регистрации без парного `unregister`: `kernel/src/lib.rs:885` (`user_as1`), `:886` (`user_as2`), `:1112` × 3 (сценарии exec-nx/kread/unmapped), `:1223` (`elf_as`), `:1301` (`CowParentAS`), `:1335` (`register_child` COW-ребёнка). Итого 6 утёкших + 2 живых demo-AS + 1 kernel = 9 | P0 (усиливает P0-8) | 16 − 9 = **7 свободных слотов**. В сочетании с P0-1 (каждый цикл fork/exit/wait теряет один AS) седьмой пользовательский `fork` вызывает `panic!` в `register_child` (`vmm/address_space.rs:1087`) из EL0. Это превращает P0-8 из «теоретического OOM» в «достижимый за 7 fork'ов отказ ядра» |
| **N-7** tombstone теряет список потоков задачи | **CONFIRMED** | `scheduler/process_table.rs:111-115` — tombstone сохраняет `removed.threads.first()`. Гейт на этом и спотыкается: `kernel/src/lib.rs:472` вытаскивает `t.threads.first()`, потому что остальных уже нет | P1 (сегодня латентно) | Как только у задачи появится второй поток, teardown по tombstone молча потеряет остальные. Ещё один аргумент за teardown на `exit` (там `Task` целиком в руках), а не на `reap` |

---

## 2. Владение ресурсами — ядро задачи

### 2.1. Диагноз

Сегодня у процесса **нет ни одного владельца ресурсов**. `Task`
(`kernel/src/scheduler/task.rs:35-49`) декларирует «Task is the unit of resource
ownership» (`:31-34`), но поле `owned_objects` (`:43`) не имеет писателя
(комментарий `:41-42`), а `address_space` (`:39`) — просто число без
владельца. Единственное место, где ресурсы进程-а освобождаются, — руками
написанный teardown в гейте (`kernel/src/lib.rs:456-488`, `:479-488`).

Отсюда и наблюдаемая картина: `waitpid`, `kill` (для удалённой цели),
`execve`-ошибка и отсутствие рейпера — это четыре разных пути, и ни один из них
не освобождает ничего. Teardown существует только в гейте, который поэтому и
зелёный.

### 2.2. Состояния задачи: `Exited` vs `Zombie`

Сейчас `TaskState` (`scheduler/task.rs:23-29`) = `Created | Running | Exited |
Zombie`, причём:

* `Task::exit()` (`:71-74`) всегда ставит **`Zombie`**;
* `Exited` присваивается **только** в `ProcessTable::remove` (`:117`) как
  маркер tombstone'а;
* `live_count()` (`:29-35`) исключает `Exited`, то есть считает «не-tombstone»;
* `children_of` (`:126-133`) по той же причине исключает `Exited`.

То есть один enum домена одновременно играет две роли: состояние lifecycle и
состояние слота хранения. Это корень проблемы N-7 и причина того, что
`live_count` определяется через значение состояния, а не через факт наличия
записи. Отдельного «процесс умер, но родитель не забрал» и «процесс умер и уже
убран» в модели нет вообще — а `sys_kill` для удалённой цели вообще не
переводит задачу в `Zombie` корректным путём (см. §2.4).

**Предлагаемая модель** (две правки, по возрастанию инвазивности):

*Шаг 1 (S, обязательный).* Убрать `Exited` из доменного enum и завести
отдельное состояние слота:

```rust
// scheduler/task.rs
pub enum TaskState { Created, Running, Zombie }   // Zombie = все потоки мертвы,
                                                 // exit_code финален, ресурсы УЖЕ освобождены
// scheduler/process_table.rs
enum Slot { Free, Live(Task), Tombstone { generation: u32 } }
```

Следствия: `live_count()` = число `Live` (нельзя обмануть значением поля);
`remove()` возвращает `Live`-задачу целиком (теardown получает полный список
потоков, N-7 закрыт); tombstone перестаёт быть «задачей, у которой
`threads.first()`» и не может случайно выглядеть как живая.

*Шаг 2 (M, по желанию).* `Running` при >1 потоке требует счётчика («сколько
потоков этой задачи сейчас исполняется»). При текущем «один поток на задачу»
это вырождается в bool. Если `clone` откладывается (§4.3), шаг 2 можно не
делать — но проектировать `TaskState` так, чтобы добавление счётчика было
локальным.

Ответ на вопрос «хватает ли `Created/Running/Exited/Zombie`»: **нет, и дело не
в количестве имён, а в двух отсутствующих вещах** — (а) в факте
`Created → Running` для user-потоков (CONFIRMED выше) и (б) в том, что
`Zombie` сейчас означает «родитель ещё не забрал», хотя различать эти вещи
нечем: ресурсы всё равно не удерживаются. Когда teardown переезжает в `exit`
(§2.3), `Zombie` автоматически становится «только код возврата».

### 2.3. Кто владеет `AddressSpace`

**Ядро.** Точнее: `Task` владеет ровно одним AS, и ядро — единственный
исполнитель владения. Ни родитель, ни ребёнок, ни пользователь AS не имеют на
него прав; это исключает класс «ребёнок умер, родитель замапил что-то в его
AS» и делает текущий AS-реестр (`vmm/address_space.rs:1057`) чистой
реализацией этого правила, а не отдельной подсистемой.

Три момента смерти, и они **разные**:

| Событие | Что происходит с AS | Почему |
|---|---|---|
| `execve` (успех) | Старый AS **заменяется**: строится новый AS в свежем root'е, в задаче меняется `address_space`, старый освобождается через `Release::AddressSpaceOnly`. Замена = одно присваивание поля после того, как новый AS полностью построен. Отказа не бывает | Транзакционность. ADR-035:116-119 уже нарисовал именно `replace_address_space(new_as)` |
| `exit` (последний поток) | AS **освобождается сразу**. `Zombie` сохраняет только `exit_code`, `parent`, `generation` | AS-пул — единственный по-настоящему тесный ресурс (16). Держать AS зомби до `waitpid` — значит позволить 16 зомби заблокировать всю систему. И это делает `waitpid` тривиальным: он ничего не освобождает, а просто читает код |
| `waitpid` (reap) | **Ничего не освобождает.** Только копирование `status` и превращение слота в tombstone | Следствие предыдущего решения. Прямо устраняет P0-1 целиком, а не «чинит» |

Это ключевой сдвиг: **teardown происходит на `exit`, а не на `reap`**. Он
убирает целый класс проблем (блокировка пула зомби, N-7, остановка
`waitpid`-а на неблокируемом пути) и делает точку смерти единственной: задача
умирает только тремя способами — последний поток вызвал `exit`, фатальный
EL0-fault, `SIGKILL`. Все три обязаны сойтись в один вызов.

Цена: зомби нельзя исследовать. Для Vivanta это приемлемо — нет `/proc`, нет
core-dump, а ADR-035:57 объявляет `cwd`/`fd_table` будущим. **Это решение надо
записать в ADR-035 явно**, иначе через пол-года кто-то «оптимизирует» teardown
обратно в `waitpid` ради симметрии с Linux и вернёт P0-1.

### 2.4. Единый teardown-путь: где он живёт

**Модуль:** новый `kernel/src/scheduler/teardown.rs` (в `scheduler`, потому что
он должен видеть и таблицу процессов, и runqueue; `vmm` не должен знать про
`scheduler` — зависимость строго односторонняя).

**Единственная публичная функция:**

```rust
// kernel/src/scheduler/teardown.rs

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Release {
    /// Последний поток задачи исчез. Освободить всё, что задача владеет
    /// (AS + маппинги + COW-фреймы + sibling-потоки + их kernel-стеки),
    /// ОСТАВИВ запись в таблице как Zombie с финальным exit_code.
    /// Стек ТЕКУЩЕГО потока не трогаем — он ещё исполняется.
    OnLastThreadExit,
    /// Замена образа: адресное пространство вытеснено новым, у старого
    /// больше нет владельца. Потоки и запись живы.
    AddressSpaceReplaced,
    /// Полное удаление записи (orphan-репер, teardown в гейтах).
    /// Ресурсов уже нет; освобождать нечего, только tombstone.
    Discard,
}

pub struct ReapReport {
    pub as_id: AddressSpaceId,
    pub mappings_dropped: u32,
    pub anon_frames: u32,
    pub cow_frames: u32,
    pub tables_reclaimed: u32,
    pub threads_removed: u32,
    pub stacks_freed: u32,
}

pub fn release_task(task_id: TaskId, how: Release) -> ReapReport;
```

**Инварианты (проверяются компилятором и комментарием, не соглашением):**

1. Никогда не переключает контекст. Это критично: `waitpid` вызывается из
   syscall-контекста, `thread_exit` — посреди shutdown.
2. Никогда не освобождает kernel-стек вызывающего потока. Собственный стек
   освобождается существующим `cleanup()` (`scheduler/mod.rs:589-619`) при
   *следующем* `thread_exit` — этот путь уже корректен и его не трогаем.
3. Вызывается при выключенных IRQ (или сам берёт `interrupts_guard`).
4. `release_task` — единственный вызывающий `vmm::unregister`,
   `AddressSpace::unmap_all` и `RunQueue::remove` в рантайме. Гейты получают
   `release_task(..., Discard)`, а не собственную реализацию. Это и есть
   механизм, который запрещает гейту снова «замаскировать» дефект.

**Корректная последовательность `OnLastThreadExit`:**

```
1.  Снимок current_tid ДО любой мутации          // mod.rs:493-495 уже так делает
2.  task.exit(code)  -> state = Zombie, exit_code = Some(code)   // идемпотентно:
                                                             // повторный выход не перетирает код
3.  SIGCHLD родителю + wake_waiters(task_id)      // mod.rs:522-529 уже так делает
4.  Отвязать задачу от родителя: parent.children.retain(|c| c != task_id)
                                                          // N-7: список потоков теряется на reap,
                                                          // поэтому чистим здесь, где Task целиком
5.  for t in task.threads, t != current_tid:  rq.remove(t); free_kernel_stack(t)
6.  AS: allocator = vmm::faults::make_allocator(as_id)   // faults.rs:60-63 — НЕ нужен &mut SystemState,
                                                    // поэтому teardown вызываем из syscall
    aspace.unmap_all(&mut alloc)     -> anon-фреймы в PMM, COW-refcount→0→PMM,
                                       reclaim_empty_tables -> реестр tables (address_space.rs:907-924)
7.  if let Some(root) = aspace.root_owned { pmm.free(root) }   // НОВОЕ поле, закрывает P1-root
8.  vmm::unregister(as_id) -> Err должен быть невозможен (маппингов уже нет)
9.  task.owned_objects: каждый MemoryObject.drop()      // writers появятся позже,
                                                   // но место в последовательности уже есть
```

Шаг 5 **обязателен** именно здесь, и он же — то, что сегодня не делает
`sys_kill`: `sys_kill` для удалённой цели пишет `ThreadState::Terminated`
(`syscall/process.rs:379-382`) напрямую и **никогда не вызывает
`thread_exit`**, значит `cleanup()` (а с ним и возврат kernel-стеков) не
запускается, а задача остаётся `Zombie` с живым AS. Правильная форма:
`sys_kill(SIGKILL)` не трогает состояния потоков вообще — он выставляет флаг
`kill_pending` в задаче и (для self-kill) вызывает `thread_exit`, а для
удалённой цели либо (а) помечает потоки Terminated и через
`wake_waiters`/следующий `thread_exit` приводит к тому же шагу 1–9, либо
(б) — проще и честнее — добавляет в runqueue поток-«killer»? Нет, (б)
некорректно. Реалистичный минимум: `sys_kill` удалённой цели вызывает
`scheduler::terminate_task(pid, -9)`, который **сам** выполняет шаги 2–9
(он не трогает текущий поток, потому что текущий — не в списке жертвы), а
затем переводит потоки жертвы в `Terminated`, чтобы `cleanup()` при ближайшем
`thread_exit` вернул их стеки. **Это один и тот же код-путь с `exit`, и
`waitpid` его не касается.**

### 2.5. Исчерпание пула: паника → возвращаемая ошибка

**API меняется так:**

```rust
// vmm/address_space.rs
pub enum VmmError { …, PoolFull }                       // +1 вариант к :1048-1055
pub fn register_child(child: AddressSpace, root: RootPageTable) -> Result<AddressSpaceId, VmmError>
pub fn register(root: RootPageTable, flags: AddressSpaceFlags) -> Result<AddressSpaceId, VmmError>
// scheduler/mod.rs
pub fn create_task_for_thread(...) -> Result<TaskId, SchedulerError>   // вместо .expect на :267
```

**Путь отказа в `sys_fork` должен быть транзакционным** — ровно так, как уже
сделан для kernel-стека: `free_child_boot_frames`
(`syscall/process.rs:19-33`) освобождает стек и root; к нему добавляется
`vmm::unregister(registered_child_as_id)`, если регистрация прошла, а `Task`
не создалась. Порядок в `sys_fork` обязан стать: стек → root → клон
kernel-половины → `duplicate_as` → **регистрация AS (теперь `Result`)** →
`Task` (теперь `Result`) → `publish_thread`. Любая точка отказа до
`publish_thread` обязана оставить систему в состоянии «как до вызова»; это
свойство и делает `publish_thread` последним шагом (см. фикс `d083f8e`).

**Какой errno.** ADR-033:54-58 замораживает ровно пять значений
(`EPERM=-1, ENOMEM=-12, EFAULT=-14, EINVAL=-22, ENOSYS=-38`) и требует
Linux-совместимого кодирования; ADR-033:37 требует, чтобы *номера syscall*
добавлялись, но про errno такого требования нет — кодирование задано
списком, а не битовой схемой. Добавление двух констант **не меняет ни одного
существующего значения**, поэтому это аддитивное расширение, а не смена
контракта. Выбор:

| Ситуация | errno | Обоснование |
|---|---|---|
| Пул AS исчерпан, таблица процессов полна | **`-EAGAIN` (-11)** | POSIX/Linux: `fork` → `EAGAIN`, «ресурс временно недоступен, повторите». Таблица процессов — это лимит ядра, а не лимит памяти; `EAGAIN` прямо говорит пользователю «дождитесь `waitpid` и повторите». `-ENOMEM` был бы ложью (память есть) и увёл бы чужой код в OOM-обработчик. `-EMFILE` (-24) — про лимит дескрипторов одного процесса; fd-таблицы в Vivanta нет (`ADR-035:59` объявляет её backlog) |
| Реальный OOM кадров (kernel-стек, root, page-table, файловые страницы) | **`-ENOMEM` (-12)** | Уже возвращается корректно: `syscall/process.rs:74, 84, 104, 123, 140`. Сохраняем |
| Плохой пользовательский указатель | `-EFAULT` (-14) | Уже корректно: `:277, 444, 476, 497, 565` |
| Плохой аргумент | `-EINVAL` (-22) | Уже корректно |
| `kill` на несуществующий pid | **`-ESRCH` (-3)** | POSIX-значение. Сейчас возвращается `EINVAL` с комментарием `// ESRCH` (`:336-337`) — комментарий прямо признаёт расхождение. Требует новой константы |
| `waitpid` без детей | `-ECHILD` (-10) | Уже корректно, и **уже поправлено** коммитом `602fe7b` |

Итого к ADR-033 нужна одна приписка: две аддитивные константы `-EAGAIN=-11` и
`-ESRCH=-3` плюс таблица «условие → errno» для новых путей. Ни один
существующий вызов не меняет поведение.

### 2.6. Транзакционный `execve`

**Ответ на прямо поставленный вопрос: «достаточно ли валидировать весь ELF до
сноса?». Нет.** Разберём, почему, и что именно ломается.

`plan_load` (`exec/src/elf.rs:198-343`) валидирует образ полностью и хорошо:
магию, класс, endianness, `e_machine`, `e_type`, границы таблицы
program headers (`:230-235`), `memsz < filesz` (`:256`), переполнения (`:260,
:265`), конгруэнтность page-alignment (`:253-255`), перекрытие сегментов после
округления (`:293-323`), W^X (`:273-275`) и «entry внутри исполняемого
сегмента» (`:325-337`). Это покрывает весь класс «битый образ».

Но `sys_execve` падает не на этом. После `unmap_all` остаются отказы,
приходящие от **целевого** AS и от PMM:

1. `reserve_at` сегмента (`:47` в `exec/mod.rs`) → `MappingTableFull` при
   `mappings.len() == 64` (`vmm/mapping.rs:147`) или `OutOfVa`;
2. `try_alloc_page_table_frame` для файловых страниц (`exec/mod.rs:59-61`) →
   OOM; там же, кстати, **неверная классификация**: возвращается
   `VmmError::OutOfVa`, хотя это не VA-истощение (`exec/mod.rs:60`);
3. `materialize_with` (`exec/mod.rs:73` → `vmm/address_space.rs:426-477`) может
   вернуть `MappingTableFull` на финальном разбиении shadow;
4. `reserve_at` стека (`syscall/process.rs:647-652`) — ещё одна точка отказа
   уже после сноса;
5. при новой схеме — `alloc_contiguous` для kernel-половины свежего root'а.

Валидация-первой — **необходимое, но не достаточное** условие. Она дешёвая и
должна быть сделана первой (превращает «битый образ» в аккуратный
`-ENOEXEC`/`-EINVAL` без единого изменения состояния), но гарантию
транзакционности даёт только другая конструкция.

**Предлагаемая конструкция — сборка в свежем AS + подмена указателя:**

```
1. plan_load(image)                         -> Err => -EINVAL, состояние не тронуто
2. alloc_frame() -> new_root;  write_bytes(0)               // как в gate_e2e_delta (lib.rs:529-531)
3. make_allocator(peek_next_as_id())
4. mmu_clone_kernel_half(kernel_root, new_root, alloc)      // ОБЯЗАТЕЛЬНО:
        // process.rs:107-112 объясняет, почему без kernel-половины
        // первый же fetch умирает: нет RAM-маппинга. Сейчас execve
        // этого не делает, т.к. работает в живом AS — при переезде
        // в свежий root это станет обязательным.
5. new_as = AddressSpace::new(0, new_root, User)  (пустой shadow)
6. load_plan(&plan, new_as, alloc)              -> Err => шаг 7 в обратном порядке
7. build_initial_stack(new_as, argv, envp)     -> шаги reserve + копирование строк
8. I-cache maintenance по ВСЕМ PF_X-сегментам (не только по entry)
9. new_id = vmm::register(new_as, User)        -> Err(PoolFull) => откат 7,6 и -EAGAIN
10. task.address_space = new_id                 // ОДНО ПРИСВАИВАНИЕ
11. записать frame: elr, spsr=EL0t, x[0..2]=argc/argv/envp, x[19..28] сохранены
12. release_task(task_id, AddressSpaceReplaced) // старый AS — единственный вызывающий
```

Ключевое свойство: **шаг 10 — единственное необратимое действие**, и оно не
может отказать. Всё до него — обратимо, и цена отката — один
`unmap_all` + `unregister` + `free_frame` на непопулярном пути.

Требуемые новые куски: `exec::load_plan(&LoadPlan, &mut AddressSpace, &mut
dyn PageTableAllocator, u64) -> Result<u64, VmmError>` (переиспользовать
`plan_load` один раз, а не два — сейчас `load_elf` планирует внутри себя,
`exec/mod.rs:40`); публичный экспорт `mmu_clone_kernel_half` уже есть
(`arch-api/src/mmu.rs:236-240`).

**Цена: +1 слот AS на каждый `execve`.** Это честный аргумент в пользу
поднятия `MAX_ADDRESS_SPACES` (§4.4): при 16 слотах и текущей утечке 9
транзакционный `execve` может упереться в пул. Поэтому порядок работ обязан
быть: сначала N-6 (починить утечки гейтов) и eager-teardown, потом
транзакционный `execve`, потом расширение пула.

**Запасной вариант (если fresh-AS отвергнут по причинам BSS):** держать AS,
но сделать порядок «валидация → preflight (проверка ёмкости shadow, VA,
пред-аллокация всех файловых фреймов в локальный список) → снос → применение
безотказной секцией». Остаточное окно отказа — выделение page-table фреймов
внутри `materialize_with`, и закрыть его нельзя. Этот вариант честно
называется «почти транзакционный» и не решает проблему; рекомендую его
отклонить, записав причину в ADR.

---

## 3. Целостность гейтов — часть, которая важнее всего

### 3.1. Каталог гейтов с вакуумным PASS

Все строки — `vivanta-boot/kernel/src/lib.rs`, если не указано иное.

| # | Гейт | Строка PASS | Вакуумный путь | Классификация |
|---|---|---|---|---|
| **V-1** | **M6 process-lifecycle** | `:1995` `[M6] process lifecycle demo OK` | Ассерты внутри `if let Some(task) = taskman.get(tid)` (`:1951-1962`). Если записи нет — **ноль проверок**, `:1995` печатается. Плюс `assert!(after_reap_free >= before_reap_free)` (`:1990-1993`) — направление выбрано так, что оно проходит при нулевом освобождении; должно быть `==`/`>` с требованием `reaped > 0`. Счётчик `reaped` (`:1976-1981`) печатается и не проверяется | **VACUOUS + WEAK.** Самый явный случай в наборе |
| **V-2** | **G3 fault-containment** | `:1075` `boot thread survived the faulting task (containment OK)` | Ни одного ассерта. Не проверяется, что faulting-task вообще дошёл до EL0, упал и получил `exit_code = Some(-1)`. Единственная последующая проверка — `if let Some(ft) = faultman.get(ftid)` (`:1963-1968`), которая **только печатает**. Если `ftid` не существует, строка `:1075` всё равно напечатана | **VACUOUS.** Печатает «containment OK» без единого измерения |
| **V-3** | **`[FORK-EL0] child teardown`** | `:489` | Не вакуумный *после* `602fe7b` (`:476-478` паникует), но **содержательно пуст**: teardown выполняет гейт (`:479-488`), а не `waitpid`. Маркер означает «waitpid снял запись из таблицы», а заявленная цель гейта — «ресурсы ребёнка освобождены». Плюс родитель не проверяет ни код выхода ребёнка, ни `status` (`arch-aarch64/src/user.rs:366`, `:370-373`) | **VACUOUS ПО СМЫСЛУ** (маскирует P0-1). Технический `if let` закрыт |
| **V-4** | **`[FORK]` синтетический** | `:1466` | Ассерты формально есть (`:1401-1463`), но: подделка `CURRENT_THREAD` без IRQ-защиты (`:1423-1464`); «процессы» — Ready-потоки с пустыми AS (`:1366, 1384`, entry `0x5E00_0000` без маппинга); выход ребёнка **не выполняется** — `Task::exit(7)` напрямую (`:1442-1445`); teardown родителя под `if let Some` (`:1469-1471`) | **UNSOUND.** Не вакуумный, но проверяет не тот слой и создаёт гонку в собственном теле гейта (см. P1 «синтетический FORK») |
| **V-5** | **`[KILL]`** | `:273` | Ассерты реальные (`:256-272`), но цель — табличная задача с **пустым** списком потоков (`:242-245` — `child_task.threads.clear()`). Цикл `sys_kill` по потокам (`syscall/process.rs:369-383`) не выполняется ни разу; ни один поток не переводится в `Terminated`; teardown удалённой цели не проверяется | **PARTIAL.** Проверяет табличную часть, мёртвую ветку потоков — нет |
| **V-6** | **`[PTABLE]`** | `:223` | Не вакуумный: `assert_eq!(live_count(), base)` (`:218-222`) после 70 циклов. Но доказывает только переиспользование tombstone'ов; ни AS, ни потоки, ни маппинги не создаются (задачи табличные, `threads.clear()` на `:211-215`) | **WEAK BY SCOPE.** Помечен в §2 как доказательство P0-1, но не является им |
| **V-7** | **M7.2 / M8.4 teardown** | `:1210`, `:1270` | `vmm::unregister(sys_as)` (`:1210`) для M7.2 — **но** AS там собрана через `mmu_map_user_pages` (`:1180-1185`, `:841-846`), то есть маппинги живут в root'е, а не в shadow → `mappings.len() == 0`, `unregister` проходит и **ничего не освобождает** (boot-эра-таблицы не в реестре, `vmm/address_space.rs:1109-1111`). Для M8.4 `unmap_all` (`:1270`) настоящий | **MISLEADING.** Строка «teardown» не означает освобождение |
| **V-8** | **G4 preemption** | `:2061` | `assert_eq!(running_thread_count(), 1)` (`:2062`) — ассерт по `ThreadState` **корректен**, не вакуумный. Но M5.0 §11 (`:736-765`), §12 (`:768-797`), §28 (`:1275-1312`) требуют доказательства чередования A↔B, частоты тиков и идентичности `current`. Счётчики `PREEMPT_COUNTER_A/B` печатаются (`:2042-2053`) и не проверяются | **WEAK vs SPEC.** Не лжёт, но не выполняет ратифицированную спецификацию M5.0 |
| **V-9** | **Protect/TLBI (`:971-1009`), Phase-10 (`:1100-1169`), [OOM] (`:290-331`), [E2E] (`:497-558), [COW] (`:1297-1356), [DUAL] (`:337-406), [EXEC2] (`:147-194), [SYS] (`:1176-1211), [ELF] (`:1217-1271)** | — | Ассерты точные, без `if let`, с ненулевыми счётчиками. `[E2E]` (`:546-556`) — эталон: точное равенство по трём реестрам. `[COW]`: отличный образец проверки «регистр вырос» (`:1686-1689` — в другом гейте) | **OK.** Эталон для остальных |

Итого: **3 вакуумных (V-1, V-2, V-3), 1 небезопасный (V-4), 1 частичный
(V-5), 3 слабых (V-6, V-7, V-8)**, остальные — корректные. Два вакуумных
гейта (**V-1, V-2**) вообще **не имеют маркера** в `qemu-gates.sh` — то есть
логика CI их не проверяет даже по косвенным признакам.

### 3.2. Структурно безопасный паттерн гейта — дизайн

Это самое высоко-leverage изменение в кластере. Требование жёсткое:
**«PASS» должен быть непечатаем, если ассерт фактически не выполнялся.**

Ключевое наблюдение о текущем коде: печать маркера и ассерты — независимые
операции в произвольном порядке (`println!` в `:1995` не связан с `assert!` в
`:1956`). Никакой статический анализ это не поймает. Значит связь должна быть
в *типе*: маркер физически недостижим без выполненной проверки.

**Предлагаемый API (новый файл `kernel/src/gate.rs`, `no_std`, без аллокаций,
без зависимостей):**

```rust
//! Vacuous-pass-proof gate helpers (kernel/src/gate.rs)

use core::cell::Cell;

/// Static description of one gate. Lives in the `GateScope`'s frame for the
/// duration of the gate body — no allocation, no statics.
pub struct GateCell {
    /// Human name, printed on failure ("M6 process-lifecycle").
    pub name: &'static str,
    /// The exact serial marker qemu-gates.sh greps for. The gate body
    /// CANNOT print this string itself: the literal exists only here, and
    /// the only path to it is `GateScope::pass()`.
    pub marker: &'static str,
    /// Minimum number of executed checks. `pass()` refuses to print if
    /// fewer ran. This is what makes "the body silently became a no-op"
    /// a failure instead of a green marker.
    pub min_checks: u32,
}

pub struct GateScope<'a> {
    cell: &'a GateCell,
    checks: Cell<u32>,
    finished: bool,
}

impl<'a> GateScope<'a> {
    pub fn check(&self) -> Check<'_> {
        self.checks.set(self.checks.get() + 1);
        Check { scope: self }
    }
    /// THE ONLY function in the kernel that can print a gate marker.
    /// Consumes self; panics if the gate body never called `check()` at
    /// least `min_checks` times.
    pub fn pass(mut self) {
        if self.checks.get() < self.cell.min_checks {
            panic!(
                "gate {}: only {} checks ran, min is {} — refusing to print marker",
                self.cell.name, self.checks.get(), self.cell.min_checks
            );
        }
        // Printed as part of the boot console's IRQ-guarded write
        // (boot_common with_console + set_console_irq_guard, lib.rs:580),
        // so a timer IRQ cannot interleave inside the marker line (M5.0 §13).
        vivanta_boot_common::println!("  {} PASS (checks={})", self.cell.marker, self.checks.get());
        self.finished = true;
    }
}

impl<'a> Drop for GateScope<'a> {
    fn drop(&mut self) {
        if !self.finished {
            // Covers: early `return`, `?`, falling off the end without
            // pass(), and a body that returns before reaching its checks.
            panic!("gate {}: body exited without pass()", self.cell.name);
        }
    }
}

/// One observation. Every method panics on violation — same fail-fast
/// semantics the current `assert!`s have, with better messages.
pub struct Check<'a> { scope: &'a GateScope<'a> }

impl<'a> Check<'a> {
    /// Exact equality. `show` is echoed in the panic message so a failing
    /// log names the measurement, not just "assertion failed".
    pub fn eq<T: PartialEq + core::fmt::Debug>(&self, what: &str, got: T, want: T) { … }

    /// A count that MUST be non-zero. This is the primitive that makes
    /// "the workload did not happen" unrepresentable, and it is what the
    /// M6 gate's `after >= before` should have been.
    pub fn nonzero(&self, what: &str, got: usize) { … }

    /// An `Option` that must be `Some`. Destructure HERE, not in an
    /// `if let`: a `None` panics, it does not skip. This is the primitive
    /// that makes the M6 gate's `if let Some(task)` shape impossible.
    pub fn some<T: core::fmt::Debug>(&self, what: &str, got: Option<T>) -> T { … }

    /// Reclamation: `after` MUST equal `before` exactly. The only form the
    /// API offers for "resources were released", so `after >= before` is
    /// unrepresentable. Pair with `nonzero("… cycles", n)` to prove the
    /// workload actually ran.
    pub fn delta_zero(&self, what: &str, before: usize, after: usize) { … }

    /// Arbitrary precondition, for the rare case a typed primitive does not
    /// fit. Same panic semantics.
    pub fn require(&self, what: &str, cond: bool) { … }

    /// Sugar used by 6 gates: a task must have exited with an exact code.
    pub fn exit_code(&self, what: &str, task: Option<&Task>, want: i32) { … }
}

/// The macro is the ergonomic entry point and the only place a marker
/// literal may appear.
#[macro_export]
macro_rules! gate {
    ($name:literal, $marker:literal, min $n:literal, |$g:ident| $body:block) => {{
        let cell = $crate::gate::GateCell {
            name: $name, marker: $marker, min_checks: $n,
        };
        let $g = $crate::gate::GateScope::new(&cell);
        $body
        $g.pass();
    }};
}
```

**Почему это работает — разбор по каждому классу вакуумного PASS:**

| Класс дефекта | Как делается невозможным |
|---|---|
| Ассерт внутри `if let`, ветка не выполнена (V-1) | Проверка не может быть написана внутри `if let`: `check().some("task", tm.get(tid))` паникует на `None`. Если автор сначала напишет `if let Some(t) = ... { … }`, то внутри не будет ни одного `check()`, и `pass()` упадёт по `min_checks` |
| Сравнение `after >= before` (V-1) | API не предлагает направленного сравнения «не меньше» для рекломации. Есть только `delta_zero` (точное равенство). `>=` физически не пишется через `GateScope` |
| `println!("PASS")` вне условия (V-1, V-2) | Строка маркера — поле `GateCell`, единственный путь к `println!` — `pass()`. Напечатать маркер из тела гейта нельзя не нарушив единственный источник истины (линтер/ревью это ловит, но главное — не остаётся API) |
| Счётчик, который может быть нулём (V-1 `reaped`, V-2 «faulting task не запускался») | `nonzero("reaped tasks", reaped)`; для V-2 — `nonzero("faulting task observed", …)` плюс `exit_code("fault task", Some(&t), -1)` |
| Гейт не дошёл до конца (ранний `return`, `?`) | `Drop`-проверка `finished` |
| Гейт выродился в no-op при рефакторинге | `min_checks` + счётчик в самой строке маркера: `[M6] … PASS (checks=7)`. Маркер с `checks=0` в логе невозможен, а ревьюер видит число |

**Дополнительное свойство, которое стоит заложить сейчас:** строка маркера
должна **меняться** при изменении гейта. Тогда старый `qemu-gates.sh`,
проверяющий старый текст, честно покраснеет, а не продолжит числиться зелёным
по инерции. Формат: `[M6] lifecycle: 2 zomibles reaped, pmm delta=0` — данные
в маркере, а не только слово «PASS».

**Стоимость:** XS–S (файл + макрос + перенос 3 вакуумных гейтов как
пилот). **Риск:** низкий. Хрупкое место одно — `min_checks` придётся
подбирать руками; это односторонне безопасно (занижение → гейт останется
слабее, завышение → гейт упадёт на бою и это заметно сразу).

### 3.3. Влияние на 18 маркеров `tools/qemu-gates.sh`

`tools/qemu-gates.sh:28-48` содержит 18 обязательных маркеров; терминальный —
`'[G4] running_count=1'` (`:48`). Логика проверки: `grep -qF` по подстроке
(`:99-106`), плюс детектор паники `'PANIC|EXCEPTION|G4 FAIL'` (`:80`, `:111`),
который ловит `panic_handler` бинарника (`target-qemu-aarch64/src/main.rs:137-143`
печатает `PANIC: `). То есть **против паники скрипт не слеп**; он слеп
именно против «PASS без проверки».

| # | Маркер (`qemu-gates.sh`) | Слабость | Что меняется |
|---:|---|---|---|
| 4 | `[FORK-EL0] fork/waitpid round-trip exit(42) PASS` (`:32`) | Родитель проверяет только `wait_ret == pid` (`arch-aarch64/src/user.rs:370-373`); ни кода выхода 7, ни `status` | Блоб передаёт настоящий `status`-указатель в свой стек, после `waitpid` читает `status` и сравнивает с `7 << 8`; при несовпадении `exit(12)`. Маркер сохраняется, но теперь означает «код возврата проверен» |
| 5 | `[FORK-EL0] child teardown PASS` (`:33`) | **Сильнее всего.** Гейт делает teardown руками, маскируя P0-1 | После появления `release_task` блоб не делает teardown; гейт **не** трогает AS ребёнка, а проверяет, что AS ребёнка уже отсутствует в реестре (`vmm::count()` вернулся к базовому) и `pmm.free_count()` вырос ровно на освобождённое. Маркер → `[FORK-EL0] child AS+stack released by waitpid PASS (checks=…)` |
| 9 | `[PTABLE] 70 create/reap cycles` (`:37`) | Слеп к AS/потокам/маппингам | Расширить гейт до `pmm.free_count()` / `vmm::count()` / `tables::total()` / `cow_refcount::total()` delta-ноль через 70 циклов **реального** fork/exit/wait (через `sys_fork`, не таблично). Тогда маркер честно называет, что проверяет |
| 10 | `[KILL] SIGKILL->Zombie + SIGCHLD PASS` (`:38`) | Цель без потоков → ветка `Terminated` не проверяется | Цель = реальная задача с реальным потоком; после `kill` ассертить, что поток стал `Terminated` и его kernel-стек вернулся (delta по PMM) |
| 15 | `[FORK] pid parent=` (`:43`) | Подделка `CURRENT_THREAD`, пустые AS, фейковый `exit(7)` | Либо удалить гейт целиком (он дублирует `[FORK-EL0]`, который честнее), либо переделать на реальный EL0 `fork`. **Рекомендация — удалить**: держать два гейта на одну функциональность, один из которых нечестный, хуже, чем иметь один честный |
| 17 | `Hello, Vivanta!` (`:45`) | Чистая лог-строка демо-задачи; не измеряет ничего | Оставить (дёшево и полезно как smoke транспорта `write`), но не считать доказательством |
| 18 | `[G4] running_count=1` (`:46`, терминальный) | Не проверяет то, что требует M5.0 §11/§12/§28 | Оставить как `running_count=1`, **добавить** `[G4] switches=N alt=A<->B ok` — из уже существующих счётчиков (`:2079-2080`), плюс проверку, что `current` менялся. Терминальным сделать новый маркер |

Маркеры 1, 2, 3, 6, 7, 8, 11, 12, 13, 14, 16 (11 штук) — усиления не требуют;
для 6, 8, 11, 12, 14, 16 достаточно обёрнуть в `gate!` с `min_checks`, чтобы
зафиксировать регресс-щит.

**Отдельно про V-1 и V-2:** они не имеют маркеров вообще. Добавить
`[M6] lifecycle: 2 zomibles reaped, pmm delta=0` и
`[G3] fault contained: exit=-1, EC=0b100100` в `MARKERS` — иначе
вакуумные гейты останутся невидимыми для CI.

---

## 4. Функциональность, которую это открывает

### 4.1. Настоящая асинхронная доставка сигналов

**Что нужно, чтобы `pending` перестал быть write-only.** Три вещи: точка
входа в EL0 для перехвата, кадр handler'а на пользовательском стеке и
`rt_sigreturn`.

**Точка врезки — уже существует и уже доказана.** SVC-эпилог
(`arch-aarch64/src/vectors.rs:137-191`) после вызова `bl el0_sync_handler`
**перечитывает `elr`/`spsr`/`sp` из кадра** (`:166-172`) перед `eret`.
Значит Rust-обработчик, переписавший живой `ExceptionFrame`, реально меняет
куда возвращается EL0. Это ровно тот механизм, который сегодня использует
`sys_execve` (`syscall/process.rs:664-670`): он переписывает `elr`, `spsr`,
`x[0..31]` в живом кадре и возвращает 0, и `eret` уходит в новый образ.
**Сигнальная врезка — тот же приём в другом месте.**

Предлагаемые три точки (по убыванию важности):

1. **`el0_sync_handler`, после `syscall_dispatch`** (`arch-aarch64/src/user.rs:504-515`).
   Покрывает возврат из любого системного вызова — самый частый путь.
2. **Ветка EL0-fault, перед `user_fault_terminate`** (`user.rs:547-559`).
   Правильный порядок: до контейнмента доставить сигнал, если он есть
   (иначе фатальный fault всегда «побеждает»); контейнмент — если доставка
   не удалась.
3. **Первый вход в EL0 через `eret_to_user_stub`** (`user.rs:19-56`) — там
   чистый asm, проверку на Rust добавить нельзя. Решение: проверять
   наличие сигнала **в точке активации потока**, то есть перед
   `context_switch` в `yield_now`/`thread_exit`
   (`scheduler/mod.rs:404-414`, `:570-579`): если у следующего потока есть
   доставляемый сигнал, заранее переписать его `ArchContext`'ный
   `ExceptionFrame` (`elr` → trampoline). Это единственное место, где
   известен и текущий, и следующий поток.

Плюс обработчик IRQ-возврата (`vectors.rs:80-135`) — там `eret` тоже идёт в
EL0, если прерывание пришло из пользовательского режима; на первом этапе
его можно не трогать, приняв «доставка не чаще, чем при следующем SVC» как
документированное ограничение (POSIX этого не запрещает — сигнал доставляется
при первой возможности).

**Кадр handler'а.** Минимальный, на пользовательском стеке, 272 байта —
ровно `ExceptionFrame` (`arch-api/src/exception.rs:11-16`, статический assert
`:23`):

```
  sp_handler  →  [0]      pretcode            (адрес trampoline'а для rt_sigreturn)
                [1..32]  x0..x30             (прерванный контекст)
                [32]     sp_user             (ПРЕРВАННЫЙ SP_EL0 — отдельный слот!)
                [33]     pc_user             (прерванный ELR)
                [34]     pstate_user         (прерванный SPSR, = 0x000 EL0t)
  sp_trampoline = sp_handler + 272
```

**Критичная тонкость, которую легко пропустить:** слот 31 кадра — это
`sp`, и для SVC-эпилога это **SP_EL1** (вектор сохраняет `add x0, sp, #34*8`
в `vectors.rs:155-156`, и эпилог восстанавливает из него `mov sp, x30`
в `:170-172`). Его нельзя перезаписывать пользовательским SP — ровно об этом
пишет комментарий `syscall/process.rs:658-663`. Поэтому пользовательский SP
лежит в **отдельном** слоте 32, а `rt_sigreturn` обязан восстановить
`SP_EL0` через `msr sp_el0, …` сам, поскольку ни один эпилог этого не делает.
Побочный эффект: раскладка фрейма не взаимозаменяема с `ExceptionFrame`
1:1 — нужен явный маппинг слотов в коде trampoline'а и в
`sys_rt_sigreturn`, с комментарием и хост-тестом.

**`rt_sigreturn` (номер 15, `kernel/src/syscall.rs:39`):** читает фрейм по
SP_EL0, восстанавливает `x[0..30]`, `elr ← pc_user`, `spsr ← pstate_user`,
`msr sp_el0, sp_user`, снимает блокировку доставленного сигнала, **не трогает
слот 31**, возвращает 0.

**Модель очереди.** `pending: Option<Signal>` (N-1) заменить на
`pending: u64` — битмаску. `send()` (`:84-93`) ставит бит, `take_deliverable()`
выбирает младший установленный и не заблокированный (SIGKILL всегда
проходит, `:106-112`), затем очищает бит и добавляет его в маску на время
работы handler'а (поле `mask` в `SigAction`, `:52`, уже существует и
сейчас не используется). Это POSIX-семантика без новых примитивов.

**`flags` (`:55`):** `SA_RESTART` можно не реализовывать, но его нужно
**научиться отбрасывать при установке** (`sys_sigaction` сейчас принимает
любые биты, `:499-504` проверяет только SIGKILL) — иначе пользователь
настраивает флаг, который молча ничего не делает. Минимум: отвергать
неизвестные биты с `-EINVAL`.

**Проверяемость на QEMU — высокая.** TCG (cortex-a53) достоверно моделирует
`eret`/`SPSR`/`SP_EL0`/маскирование, поэтому проверяемо: вход в handler,
правильные `x0`/`sp`/`pretcode`, восстановление контекста в `rt_sigreturn`,
`blocked`-маска (сигнал не доставляется, пока заблокирован; доставляется при
`rt_sigprocmask` — номер 14, сейчас `-ENOSYS` на `:133`, это первое, что надо
оживить), `SIGKILL` как неблокируемый, `SIGCHLD` по факту выхода ребёнка,
и комбинация «сигнал приходит во время блокирующего `waitpid`» (заодно
закрывает N-3: доставка должна прерывать ожидание). **Не проверяемо на QEMU:**
cache/TLB-аспекты доставки, FP/SIMD-контекст handler'а (ядро не сохраняет
`d8-d15` — отдельный P0-silicon из аудита, поэтому handler-ы в гейтах обязаны
быть целочисленными, и это надо записать в гейт).

**Порядок работ:** сначала `pending: u64` + `rt_sigprocmask` (S, чисто
инфраструктурно, проверяемо без доставки), потом trampoline + `rt_sigreturn`
(M), потом `SIGKILL`-через-очередь вместо синхронного `thread_exit`
(в `sys_kill` сейчас есть явный комментарий, что очередь не работает и
убийство применяется немедленно — `syscall/process.rs:371-375`; с доставкой
этот комментарий становится неверным и код должен упроститься), потом
handlers для DFL/IGN (`:13-14` уже определены, `send()` их уважает на `:88-90`).

### 4.2. `argv`/`envp` и начальный пользовательский стек

**Точная раскладка** (System V / Linux AArch64, снизу вверх):

```
   stack_top                       16-байт выровнено   ← сюда ставится SP_EL0
   │
   │  строки envp[n]..envp[0]      "PATH=/bin\0" ...
   │  строки argv[m]..argv[0]      "/init\0" ...
   │  auxv: (AT_NULL, 0)           16 байт; далее пары
   │  envp[0] .. envp[n-1]
   │  NULL                         envp[n]
   │  argv[0] .. argv[m-1]
   │  NULL                         argv[m]
 sp_0 = 16-выровненный адрес argv[0]
```

Регистры на входе: `x0 = m` (argc), `x1 = &argv[0]`, `x2 = &envp[0]`.
(Строка `auxv` — 2×u64 на AArch64, `AT_NULL = 0`; для статического
`ET_EXEC` без AT_PHDR полный auxv необязателен, но завершающая пара
обязательна, иначе libc-инициализатор, если он появится, зациклится.)

**Алгоритм ядра** (в `sys_execve`, до записи кадра):

1. `copy_user_string` (`syscall/process.rs:557-574`) для каждого `argv`/`envp`
   в ядерный буфер. Уже существует, уже валидирует диапазон.
2. `reserve_at(STACK_TOP - 65536, 65536, user|RW, OBJ_ANONYMOUS)`, где
   `STACK_TOP = USER_VA_END = 0x4000_0000` (`vmm/address_space.rs:31`).
   Сейчас резервируется **одна** страница `0x3FFF_F000`
   (`syscall/process.rs:644`) — этого не хватит ни на строки, ни на
   frame, ни на стек Rust-программы. Растём до 64 KiB: этого хватает на
   272-байтный signal-frame, 7-инструкционный `_start` и
   `core::arch::asm!`-хвост.
3. Guard-страница: `decode_prot` требует `PROT_READ` (`kernel/src/syscall.rs:74-76`),
   то есть `PROT_NONE` выразить нельзя, а `reserve_at` всегда создаёт
   `LazyAnonymous` (`vmm/address_space.rs:286`). Значит guard требует
   нового примитива `reserve_at_reserved` (или флага в `reserve_at`),
   использующего уже существующий `Mapping::reserved` (`vmm/mapping.rs:131`).
   Это маленькое добавление в VMM; **без него переполнение стека молча
   затрёт соседний VA** вместо fault. Отметить как обязательную часть
   работы, а не как polish.
4. Посчитать раскладку, записать строки и указатели через
   `user_memory::copy_to_user` — он валидирует диапазон по активному AS.
5. Записать в живой кадр: `x[0]=argc`, `x[1]=&argv[0]`, `x[2]=&envp[0]`,
   `x[3..19]=0`, **`x[19..28]` — сохранить** (N-2: сейчас
   `(*frame).x = [0u64; 31]` на `:667` затирает callee-saved).
6. `msr sp_el0, stack_top` — **вне** кадра (`:669`, комментарий `:658-663`
   объясняет, почему слот `sp` трогать нельзя).

**`user-init` и проблема курицы и яйца.** Сейчас
`user-init/src/main.rs:14-96` — на 100% голый asm, и комментарий `:5-8`
объясняет причину: «нет стека до того, как он отображён». С построением
стека в `execve` причина исчезает — нужно, чтобы **системный загрузчик
поставил SP_EL0 до первой инструкции образа**, что он теперь и делает. Остаётся
ровно одно ограничение: **первые инструкции `_start` обязаны быть asm**,
потому что пролог Rust использует стек. Минимальный шим, 7 инструкций:

```asm
.global _start
_start:
    mov  x29, xzr          // frame pointer = 0
    mov  x30, xzr          // LR не используется на EL0
    mov  x19, x0           // argc
    mov  x20, x1           // argv
    and  sp, sp, #-16
    bl   rust_main
    mov  x8, #2            // SYS_EXIT
    svc  #0
```

`rust_main` может быть обычным Rust: `#![no_main]` + `#[unsafe(no_mangle)] pub
unsafe extern "C" fn rust_main()`, читающий `argc`/`argv` и печатающий их
через `write(1, …)`. Оговорка: ELF-сегмент `.text` тогда превышает одну
страницу? Скорее всего нет (init — это десятки байт), и это надо проверить
разбором `user-init.elf` после переписывания — но если превысит, это
*идеальный* тест-кейс для исправления cache-maintenance из P0-silicon-3
(см. §4.4).

**Что это даёт по функциональности:** появляется первый программируемый
userland. Сегодня ни один образ не может получить аргументы — значит
невозможны ни `init`, принимающий имя задачи, ни `sh`, ни любой инструмент.
Вместе с §4.1 (сигналы) и §2 (транзакционный execve) это превращает
`execve` из «загрузчик одного захардкоженного образа» в примитив, поверх
которого строится всё остальное.

**Побочная правка гейта.** Блоб `execve_code_start`
(`arch-aarch64/src/user.rs:324-343`) сейчас передаёт `argv = NULL, envp = NULL`
(`:330-331`). Чтобы гейт доказывал новое поведение, блоб должен собрать
`argv` на своём же стеке (у гейта M10.3 стек есть — `build_root(... Some((src,
len, 0x5D02_0000, 0x5D03_0000))` на `kernel/src/lib.rs:99-104`, и
`mmu_map_user_pages` его мапит, `:841-846`), вызвать `execve` с непустым
`argv`, а новая программа (`user-init`) при `argc != 2 || argv[0] != "/init"`
должна выходить с **43** вместо 42. Тогда маркер
`[EXECVE] … -> exit(42)` начинает означать «execve построил корректный
начальный образ», а не «execve не упал».

### 4.3. `clone` / создание потоков

`Task.threads: Vec<ThreadId>` (`scheduler/task.rs:40`) существует; второй
поток не добавляет никто — `Task::new` делает `threads: vec![thread_id]`
(`:57`), `create_task_for_thread` (`scheduler/mod.rs:250-268`) идёт тем же
путём. `thread_exit` уже содержит логику «последний поток задачи»
(`:500-533`), то есть наполовину модель готова.

**Аргумент «не сейчас»** (и он сильный):

1. Всё, что `clone` требует, — это ровно то, что эта миссия строит и чего
   сейчас нет: владение AS с явной точкой смерти, единый teardown,
   per-thread kernel-стек, per-thread signal-frame и `rt_sigreturn`. Ввести
   второй поток раньше teardown — значит удвоить число не-владельцев AS.
2. Второй поток немедленно открывает заново вопрос «чей `SP_EL0`», «чей
   `Task.address_space`», «как считать COW-refcount при двух потоках в одном
   AS» и «кто освобождает AS — задача или поток». Ответ «задача» должен быть
   зафиксирован в teardown **до** появления `clone`, а не после.
3. Системные вызовы ADR-033 не резервируют под это место: заняты 0–15
   (`kernel/src/syscall.rs:24-39`), `clone(flags, stack, ptid, ctid, tls)`
   требует 5 аргументов — ровно `x0..x5`, места хватает, но **номер 16 надо
   занять сейчас**, чтобы его не сожгли под что-то другое.
4. M5.0 §16 (`:878-897`) замораживает «new syscalls» и «signal delivery».
   Формально заморозка относится к M5.0, но она ратифицирована как базовая
   спецификация (см. §27 `:1240-1271`), и её нарушение без явного решения —
   это именно тот «дешёвый зелёный» (`M5.0 §31`, `:1372-1401`), который
   запрещён.
5. Гейт-прогон для `clone` сейчас был бы невозможен честно: второй поток
   должен делить AS с первым, а это требует gate-а на COW/ownership, которого
   ещё нет.

**Решение: вне scope сейчас, но не закрывать.** Три обязательных
«не запечатывает дверь» решения, которые надо принять в этой миссии:

* teardown формулируется как «освобождает AS, когда **последний поток**
  задачи умер» (`Release::OnLastThreadExit` уже назван так) — не «когда
  умер единственный поток»;
* `TaskState::Running` проектируется так, чтобы превратиться в счётчик
  (§2.2, шаг 2);
* в `kernel/src/syscall.rs` завести `pub const SYS_CLONE: u64 = 16;` с
  комментарием `// reserved: clone() — not implemented` и arm
  `SYS_CLONE => ENOSYS`, как это уже сделано для `RT_SIGRETURN`
  (`syscall.rs:134`).

Гейт, который докажет готовность к `clone`, можно построить уже сейчас и
это дёшево: **проверить, что `thread_exit` при двух фиктивных потоках в
`Task.threads` переживает смерть одного и корректно освобождает ресурсы
только на втором.** Это тест кода teardown, не новой функциональности.

### 4.4. `MAX_TASKS=64` против `MAX_PROCESSES=256`

ADR-035:102 утверждает `MAX_PROCESSES = 256`; код — `MAX_TASKS = 64`
(`scheduler/task.rs:21`). ADR-035 остаётся `Proposed` (`:4`).

**Размеры (вычислено вручную по прочитанным структурам; точные байты
проверяются существующей печатью `[SIZE]` в `kernel/src/lib.rs:696-701`):**

* `SigAction` = `{u64, u64, u32}` = 24 Б (`signal.rs:49-56`);
* `SignalState` = 8 (`blocked`) + 768 (`[SigAction; 32]`, `MAX_SIG = 32`
  на `signal.rs:16`) + прочее ≈ **784 Б**;
* `Task` (`task.rs:35-49`) ≈ **912 Б**, из которых **768 — таблица
  обработчиков**, то есть 84% структуры — это поле, которое почти всегда
  пустое (реально используются 6 сигналов из 32);
* `Mapping` ≈ 64 Б, `MappingSet` = `[Option<Mapping>; 64]`
  (`vmm/mapping.rs:147`) ≈ 4.5 КБ — совпадает с assert
  `size_of::<MappingSet>() <= 5 * 1024` (`vmm/address_space.rs:13`);
* `AddressSpace` ≤ 12 КБ (assert `vmm/address_space.rs:12`), пул
  `ADDRESS_SPACES: [Option<AddressSpace>; 16]` (`vmm/address_space.rs:1057`) →
  **до ~190 КБ BSS**, что совпадает с комментарием «~10 KiB/slot»
  (`:17-18`);
* **kernel heap = 65536 байт** (`kernel/src/lib.rs:672`).

**Вывод: `MAX_TASKS = 64` — правильное текущее значение, а 256
недоступно.** 256 × 912 Б ≈ 233 КБ одних `Task` против 64 КБ heap, плюс
Vec-рост с удвоением, плюс 256 AS × ~10 КБ = 2.5 МБ BSS, плюс
256 × 64 маппингов × ~72 Б ≈ 1.2 МБ теней. 64 × 912 ≈ 58 КБ — уже
на пределе 64-КБ heap, и это при том, что ядро активно аллоцирует
`Vec` на пользовательских и fault-путях (отдельный P1 из аудита,
`docs/audit/2026-09-24-global-audit.md:107-108`).

**Но настоящий предел — не `MAX_TASKS`, а `MAX_ADDRESS_SPACES = 16`.**
Именно AS ограничивает число *живых пользовательских процессов*, и, как
показано в N-6, 9 слотов из 16 уже заняты к моменту первого `fork` из
userspace. Порядок решений:

1. **Сначала** устранить N-6 (починить 6 утечек AS в гейтах) и довести
   eager-teardown. После этого «живые» AS = «живые процессы».
2. **Затем** поднять `MAX_ADDRESS_SPACES` до 32. BSS ≈ 320 КБ — приемлемо
   для плат с ≥1 МБ RAM (комментарий `vmm/address_space.rs:17-18` уже
   аргументирует 16 тем же соображением), для QEMU с 512 МБ тем более.
3. **Затем** сжать `Task`: `SigAction` → `{handler: u64, mask: u32, flags: u32}`
   (16 Б, экономия 256 Б) и заменить `[SigAction; 32]` на 8 записей или
   на `Vec` (пустой в 95% задач). Это даёт `Task` ≈ 250–300 Б, и только
   после этого 256 становится обсуждаемым.
4. **Затем** поднять kernel heap (это отдельная задача, общая с P1
   «Vec-аллокации на пользовательских путях»).
5. **Поправить ADR-035**: зафиксировать, что `MAX_PROCESSES` определяется
   пулом AS, а не таблицей задач; перевести ADR из `Proposed` в
   `Accepted` с ссылкой на фактическую реализацию (`ProcessTable`
   с поколениями, `fork` с COW, `waitpid`, `kill`, `execve` — всё это уже
   есть, чек-лист acceptance-критериев `:136-147` можно закрывать).

Оговорка: пункт 3 меняет публичную раскладку `SigAction`, а она уже
выставлена в userspace через `rt_sigaction` (`sys_sigaction`,
`syscall/process.rs:414-517`, копирует `size_of::<SigAction>()`). Значит
это **изменение ABI**, и его нельзя делать молча: либо оставить
`#[repr(C)] {handler, mask, flags: u32, _pad: u32}` (16 б, ABI-совместимо по
первым 20 байтам при `mask` как u64 → нет, несовместимо) — то есть
аккуратно: `handler: u64, mask: u64, flags: u32` — это 20 байт полезной
нагрузки, и 24 байта с хвостом. Менять раскладку `mask` с `u64` на `u32`
нельзя, потому что `MAX_SIG = 32` требует 32 бит, а POSIX ABI определяет
`unsigned long`. Значит правильная экономия — **не** в `SigAction`, а в
числе записей: хранить таблицу на 32 записи только для задач, которые
реально вызывали `rt_sigaction` (lazy allocation, например
`Option<Box<[SigAction; MAX_SIG]>>`). Это и ABI-безопасно, и снимает 768 Б
у абсолютного большинства задач.

---

## 5. Порядок, параллелизм, риск

### 5.1. Дорожки (lanes) и точки сериализации

**Точки сериализации (жёсткие — два агента на них одновременно конфликтуют):**

| ID | Файл | Почему он общий | Кто владеет |
|---|---|---|---|
| **S1** | `vivanta-boot/kernel/src/scheduler/mod.rs` | Точка входа teardown, `Created→Running`, фикс idle-потока, хук активации для сигналов | Только Lane A. Lane D получает хук **после** A |
| **S2** | `vivanta-boot/kernel/src/vmm/address_space.rs` | `register`/`register_child` → `Result`, новое поле `root_owned`, `reserve_at_reserved` для guard-страницы | Только Lane A (ранняя часть). Lane C (execve) **зависит** от API A |
| **S3** | `vivanta-boot/kernel/src/syscall/process.rs` | `waitpid`/`kill` (A) и `execve` (C) — один файл | Либо A целиком, затем C; либо делить по функциям с жёсткой границей строк и запретом трогать шапку/импорты |
| **S4** | `vivanta-boot/kernel/src/lib.rs` | **Все** гейты живут в одном теле `kernel_main`; любая правка гейта — текстовый конфликт | Только Lane B. Lanes A/C/D **подают** новые гейты как спецификацию, B их реализует |
| **S5** | `vivanta-boot/tools/qemu-gates.sh` (массив `MARKERS`) | Список маркеров — один массив | Только Lane B |
| **S6** | `vivanta-boot/kernel/src/signal.rs` + arm `SYS_RT_SIGRETURN` в `kernel/src/syscall.rs` | Модель очереди и диспетчер | Только Lane D |

**Дорожки:**

| Lane | Содержание | Файлы | Зависит от | Effort | Риск |
|---|---|---|---|---|---|
| **E** | Усиление ELF-планировщика: диапазоны `PT_LOAD` строго внутри `[USER_VA_BASE, USER_VA_END)`; запрет `PF_W|PF_X` кодом-ошибкой `WExec` вместо `BadSegmentSize`; проверка `p_filesz`/`p_memsz` на нулевые сегменты; **хост-тесты** в `exec/src/elf_tests.rs` (там уже есть `elf_tests.rs`, 185 строк) | `exec/src/elf.rs`, `exec/src/elf_tests.rs` | ничего | S | **Нулевой конфликт. Лучший первый заход Lane** — работает на хосте, проверяется без QEMU, и делает `execve`-транзакционность дешевле (план известен до сноса) |
| **B** | `kernel/src/gate.rs` + макрос + перенос **трёх** вакуумных гейтов (V-1, V-2, V-3) + фикс V-4 (удалить или переделать) + новые маркеры в `qemu-gates.sh` | `kernel/src/gate.rs` (**новый**), `kernel/src/lib.rs`, `tools/qemu-gates.sh` | ничего | M | Низкий. **Риск, который надо назвать:** перенос гейтов на новый API одновременно *включает* V-3, то есть до появления teardown гейт M10.4 начнёт честно падать. Это **ожидаемо и правильно**: либо принять временно-красный статус и починить teardown следующим шагом, либо помечать маркер как `expected-fail` в `qemu-gates.sh` с явным списком (плохо — это ровно то, что мы чиним). Рекомендация: сделать B **одновременно** с A, а не раньше |
| **A** | Владение ресурсами: `Slot` вместо `Exited` (task.rs + process_table.rs), `release_task` (новый `scheduler/teardown.rs`), `thread_exit` через неё, `kill` через неё, eager-teardown, `root_owned`, `register*` → `Result`, `-EAGAIN`/`-ESRCH`, фикс утечек AS в гейтах (N-6), фикс `spawn_kernel`-утечки, фикс idle-потока, `Created→Running` для user-потоков | `scheduler/{mod,task,task_manager,process_table}.rs` (**новый** `scheduler/teardown.rs`), `vmm/address_space.rs`, `syscall/process.rs`, `arch-aarch64/src/user.rs` (idle) | E не требуется | **L** | **Средний-высокий, и это самый рискованный пункт миссии.** Три конкретных источника: (1) eager-teardown ломает гейт M10.4 в его нынешнем виде (`kernel/src/lib.rs:472` читает `t.address_space` из tombstone и затем `address_space_mut_by(child_as)` — при уже освобождённом AS это `panic!` на `vmm/address_space.rs:1197`); это **ожидаемо** и означает, что гейт наконец перестанет врать, но порядок «A сначала, B потом» даёт зелёный промежуточный результат, а «B сначала» — красный. Рекомендация: A и B в одной итерации Lane B+совместно, порядок внутри — A-код, потом перенос гейта; (2) `sys_fault_terminate` → `thread_exit` вызывается из обработчика исключения, и teardown внутри него трогает реестры, к которым может обращаться IRQ — нужен `interrupts_guard`. **Поправка 2026-09-29:** `disable_interrupts()` действительно маскирует IRQ (`DAIFSet #2` = бит I), так что guard защищает (прежняя формулировка «реально маскирует только SError, то есть не защищает» была основана на неверном чтении `imm4`); при этом корректность всё равно следует обеспечивать тем, что обработчик IRQ не трогает реестры процессов — это надо проверить и записать; (3) изменение формы `sys_kill` (сбросить `kill_pending` вместо `thread_set_state(Terminated)`) меняет наблюдаемое поведение гейта `[KILL]` (V-5) |
| **C** | Транзакционный `execve`: `load_plan` в `exec/mod.rs`, kernel-half клонирование, guard-страница (`reserve_at_reserved`), начальный стек 64 KiB + `argv`/`envp`, сохранение `x19–x28`, cache-maintenance по всем `PF_X`-сегментам | `exec/mod.rs`, `syscall/process.rs` (S3), `vmm/address_space.rs` (S2), `arch-aarch64/src/user.rs` (блоб execve), `user-init/src/main.rs` + `user-init/user-init.ld` | **A** (нужен `register` → `Result` и `root_owned`), **B** (гейт) | **L** | Средний. (1) `+1` слот AS на `execve` — при пуле 16 и 9 занятых это упрётся в `PoolFull`; порядок «A (включая N-6) → затем C» обязателен. (2) `user-init` на Rust меняет ELF-образ, а `user-init.elf` вкомпилирован в ядро через `include_bytes!` (`syscall/process.rs:545`) — образ пересобирается на каждой правке, это замедлит цикл, но безопасно. (3) cache-maintenance **не проверяем на QEMU** (P0-silicon-3): гейт может проверить только покрытие диапазона, и это надо честно назвать в описании гейта, иначе получится ещё один маркер, который ничего не доказывает |
| **D** | Сигналы: `pending: u64`, `rt_sigprocmask`, trampoline + signal-frame, `rt_sigreturn`, доставка в трёх точках, реконструкция `sys_kill` | `signal.rs` (S6), `kernel/src/syscall.rs` (S6, новый arm 15 + arm 14), **новый** `kernel/src/signal_frame.rs`, `arch-aarch64/src/user.rs` (**конфликт с C и A** — те же блобы), `kernel/src/lib.rs` (S4) | хук активации — **A** (S1) | **M–L** | Средний. Главный риск — **не** в сигналах, а в том, что `arch-aarch64/src/user.rs` нужен сразу трём дорожкам (A: idle/фейковые потоки; C: блоб execve; D: точки врезки). Это **вторая точка сериализации после `lib.rs`** и её надо назвать явно: **S7 = `arch-aarch64/src/user.rs`**, владелец — Lane B, остальные подают правки |
| **F** | Ёмкости: ленивая таблица обработчиков в `Task`, рост heap, рост `MAX_ADDRESS_SPACES`, поправка ADR-035 | `scheduler/task.rs`, `signal.rs`, `vmm/address_space.rs`, `kernel/src/lib.rs` (heap), ADR | **A** | M | Низкий по коду, средний по последствиям: рост heap меняет потребление памяти на всех платформах (512 МБ QEMU, 1 ГБ+ платы) и требует проверки на самом маленьком таргете |
| **G** | `kill` без полномочий: `Task.uid` (все 0 на текущем этапе), проверка «self || parent || same-uid» → `-EPERM`, `sys_sigaction` на чужой задаче — запретить | `scheduler/task.rs`, `syscall/process.rs` | **A** (нужен доступ к `Task` после teardown-reshape) | S | Низкий. Сознательно **последним** из P1: без `Task`-identity это фиктивно, а фиктивная проверка полномочий хуже её отсутствия — она создаёт ложное ощущение защищённости |

### 5.2. Рекомендуемый порядок запуска

```
Шаг 0 (параллельно, без конфликтов):  Lane E  ─┐
                                            ├── независимы
Шаг 0:                                      Lane G-спека (проектирование uid, без кода)
                                            │
Шаг 1:  Lane A  (владение ресурсами)  ───────┤  ← критический путь
        + Lane B переносит только V-1 и V-2  │     (S1,S2,S3,S4)
        + Lane D делает только `pending:u64`  │     (без доставки, без user.rs)
                                            │
Шаг 2:  Lane B переносит V-3, V-4 (требует A)┤
        Lane C (требует A)                   │
        Lane D: rt_sigprocmask + frame        │
                                            │
Шаг 3:  Lane F, Lane G, ADR-035/ADR-033      │
        Lane D: точки врезки (требует S1)    │
```

Критический путь — **Lane A**. Всё остальное либо независимо (E), либо
ждёт A (C, G), либо может идти первыми волнами в урезанном виде (B на V-1/V-2,
D на модели очередей).

### 5.3. Что не трогать в этой миссии

* **Маскирование прерываний** (`arch-aarch64/src/interrupts.rs:13-28, 45`) —
  **НЕ блокер (поправка 2026-09-29).** Прежняя запись (P0-silicon-1 аудита)
  утверждала, что `DAIFSet #2` маскирует SError, а IRQ-маскирование сломано.
  Это неверно: `imm4` кодируется bit3=D, bit2=A (SError), bit1=I (IRQ),
  bit0=F, поэтому `DAIFSet/Clr #2` корректно маскирует IRQ, а `PSTATE.I`
  (бит 7) ею и управляется; `#4` ставил бы SError-маску. Значит guard'ы
  teardown-пути **работают** и защищают критические секции от IRQ; опираться
  на них можно. (См. `20-memory-correctness.md`, а также поправки в аудите и
  `40-silicon-blockers.md`.) Внешней зависимости здесь нет.
* **COW-ownership для `External` маппингов** (P0-2 аудита,
  `vmm/address_space.rs:606-642`) — влияет на teardown (через
  `replace_slots` → `release_cow_frame`, `vmm/mapping.rs:305-308`), но
  чинится отдельно и не блокирует единый teardown-путь.
* **Сохранение FP/SIMD** (P0-silicon-4) — handler-ы сигналов обязаны быть
  целочисленными; это ограничение надо записать в гейт, а не «чинить
  попутно».
* **Правка `arch-api/src/syscall.rs:18-27`** (неверная декларация
  `syscall_dispatch` на 8 параметров вместо 9, из аудита) — тривиально, но
  это отдельный файл без конфликтов; взять как «пока мы тут» не стоит,
  чтобы не размывать Lane.

---

## Приложение: сводка «что уже исправлено / что исправлено не полностью»

Для отчётности перед аудитом:

**Уже исправлено (коммиты после 2026-09-24):**

1. `d083f8e` — поток не публикуется `Ready` до появления `Task`
   (`scheduler/mod.rs:276-278`, `task_manager.rs:78,112`,
   `syscall/process.rs:179`). Закрывает P1 «гонка публикации» полностью для
   всех трёх путей создания. **Не полностью:** `spawn_kernel` при полной
   таблице возвращает поток, но не его kernel-стек
   (`task_manager.rs:100-103`).
2. `602fe7b` — `let-else` вместо `if let` в `[FORK-EL0] child teardown`
   (`kernel/src/lib.rs:476-478`) + `waitpid` возвращает `ECHILD` вместо
   `EINVAL` (`syscall/process.rs:252, 298`). **Не полностью:** родитель не
   проверяет код выхода и `status`; teardown по-прежнему делает гейт.
3. `82a2b94` (`VaAllocator::reserve`), `4ddbbfe` (DAIF), `465cbf4`
   (`CURRENT_THREAD`) — вне нашего кластера, но упомянуты, потому что
   `reserve` — предпосылка транзакционного `execve` (§2.6, шаг 7).

**Не исправлено, но формулировка аудита неточна:**

1. P1 «`TaskState::Created → Running` не бывает» — severity завышена: единственные
   потребители — две негейтовые печати; G4-счётчик корректен.
2. P1 «синтетический `[FORK]`» — не «маскирует дефект», а «проверяет не тот
   слой и содержит гонку в собственном теле»; хуже и проще, чем казалось.
3. P0-silicon-3 «cache только для страницы входа» — на текущем
   `user-init.elf` (один `PT_LOAD`, `filesz = 0xf8`) дефект **не
   воспроизводится**; он становится наблюдаемым только на образе с
   `.text > 4 KiB`.
4. P1 «внутренние root-фреймы не возвращаются» — точная формулировка:
   они не возвращаются **и не могут быть возвращены** существующим
   `vmm::unregister`, потому что его контракт (`:1109-1111`) ошибочно
   утверждает, что root-фрейсы всегда boot-аллокированы.

**Найдено сверх аудита (N-1…N-7 в §1.1):** потеря второго сигнала в
`pending`; затирание callee-saved `x19–x28` в `execve`; livelock
блокирующего `waitpid`; idle-поток с `entry = 0` как вооружённая ловушка;
неверные errno в `mprotect`/`munmap`/`kill`; 9 из 16 слотов AS заняты
самими гейтами; tombstone теряет список потоков задачи.
