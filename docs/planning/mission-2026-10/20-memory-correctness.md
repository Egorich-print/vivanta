# 20 — Корректность подсистемы памяти: верификация аудита, фикс-дизайн, тесты, QEMU-гейты

Дата: 2026-09-29. Область: `vivanta-boot/kernel/src/{vmm,memory,pmm.rs}` +
`vivanta-boot/vm/`. Источник бэклога: `docs/audit/2026-09-24-global-audit.md`
(«Открытый бэклог» → P0 2, 3, 6, 7, 9; P1 4 пункта; P2 2 пункта).

Метки утверждений:

- **CONFIRMED** — код прочитан, механизм воспроизводим по `path:line`.
- **INFERRED** — вывод из прочитанного кода; на QEMU/хосте напрямую не выполнял.
- **UNCERTAIN** — определить не удалось; сказано прямо, что именно.

Проверено запуском на хосте: `cargo test -p vivanta-vm -p vivanta-exec --target
aarch64-apple-darwin` → 12 + 10 зелёных; `cargo test --workspace --target
aarch64-apple-darwin` → 3 + 3 + 10 + 12 = 28 зелёных. QEMU-гейты не
прогонялись (`qemu-system-aarch64` недоступен).

---

## 0. Что уже исправлено / что аудит описал неточно

| Что | Вердикт |
|---|---|
| P0-4 `VaAllocator::reserve()` | **ALREADY FIXED** (коммит `82a2b94`, после аудита). `vm/src/va.rs:143-149` требует покрытия одним свободным интервалом; `vm/src/va.rs:157-159` проверяет ёмкость до мутации. Тесты `va_tests.rs:301,321`. В списке задания его нет — отмечено, чтобы никто не переделывал. |
| «22 host-теста в `vivanta-boot/vm/src/va_tests.rs`» | **МИСДЕСКРИПЦИЯ задания.** В `va_tests.rs` ровно **12** тестов (`rg -c '#\[test\]' vm/src/va_tests.rs` → 12; строки 89, 108, 136, 149, 158, 172, 219, 280, 301, 321, 350, 364). «22» — это 12 (`vivanta-vm`) + 10 (`vivanta-exec`); то же число в `docs/audit/2026-09-25-cleanup.md:93`. |
| «`disable_interrupts()` пишет `DAIFSet #2`, то есть маскирует SError, а не IRQ» | **АУДИТ НЕВЕРЕН.** (К моменту поправки 2026-09-29 комментарий `arch-aarch64/src/interrupts.rs:13-21` уже приведён к верной раскладке; ранее он противоречил сам себе — строка верно писала раскладку, а следующая утверждала обратное.) Поле `imm4` инструкций `DAIFSet`/`DAIFClr` кодируется **bit3=D, bit2=A (SError), bit1=I (IRQ), bit0=F** (PSTATE: D=[9], A=[8], I=[7], F=[6], зеркало в `imm4[3:0]`). Значит `#2` = 0b0010 = бит 1 = **I (IRQ)**, а `#4` = бит 2 = **A (SError)**. Linux arm64 `local_irq_save` использует ровно `daifset, #2`. Следствие: P0-железо-пункт 1 и «пункт 1» ночной секции аудита («прерывания фактически не маскируются») построены на неверном чтении; в эту миссию не входят. Для меня важно обратное: **IRQ реально маскируются**, поэтому спин-локи `KernelHeap` не переиспользуются реентерабельно из таймерного IRQ, и P1-d — это риск OOM/бюджета, а не дедлок. Передать IRQ-миссии. |
| `verify_pmm` («Инвариант 2») | **ТАВТОЛОГИЯ, ничего не проверяет.** `kernel/src/verify.rs:25-28` сравнивает `allocated + reserved + free_count()` с `total_frames()`, а `free_count()` определён в `kernel/src/pmm.rs:184` как `total - allocated - reserved`. Сумма тождественно равна `total`. Гейт зелёный всегда. |
| `reserve_at` и выравнивание `size` | Частично в порядке: `size % 4096` явно не проверяется (`address_space.rs:280`), но отсекается ниже в `VaRegion::new` (`vm/src/va.rs:42-44`). Реальный пробел — в `map_pages`, см. P1-c. |
| `protect()` off-by-one в проверке границы | Не баг сегодня, но хрупко: `address_space.rs:971` пишет `affected[n]` **до** `assert!(n <= affected.len())`. Образоваться OOB не может только потому, что `MappingSet` физически ≤ 64 элементов (`mapping.rs:147`) и цикл делает ≤ 64 итераций. В `unmap_pages` тот же порядок исправлен (`:159-162`). |
| P0-1 аудита (waitpid не освобождает ресурсы) | **CONFIRMED, вне моего объёма.** Подтверждено тем, что `vmm::unregister` не вызывается ни из одного syscall-пути (`rg -n "unregister\(" kernel/src` → только `kernel/src/lib.rs`, boot-гейты). Важно для меня: P1-b — его половина, лечатся одним teardown в слое процесса. |

---

## 1. Таблица верифицированных дефектов

Серьёзность — **фактическая** на текущем дереве, не заявленная в аудите.

| ID | Статус | Evidence | Фактическая серьёзность | Фактический радиус |
|---|---|---|---|---|
| **P0-2** External → CoWShared в `duplicate_as` | **CONFIRMED** | `vmm/address_space.rs:617-619` (`parent_piece.phys = CoWShared{refcount:2}`) без ветки по `m.phys`; реестр `address_space.rs:641`; освобождение `vmm/faults.rs:22-28,97-100`; нарушение `ADR-032:142-144` и `ADR-031:108-115` | **P1 сегодня, P0 при первом file-backed/shared mmap** | Строго: ни один syscall-путь не создаёт `Present(External)` в форкабе AS. `sys_mmap` → `reserve_lazy` (`syscall.rs:189`); execve → `load_elf` → `materialize_with` → `PhysOwnership::Anonymous` (`address_space.rs:458-464`). Единственное живое место в дереве — boot-гейт COW `lib.rs:1317` (`map_pages` → External) → `duplicate_as` `lib.rs:1332`, где кадр случайно пришёл из `pmm.alloc_frame()` (`lib.rs:1302`), поэтому двойное владение сегодня *случайно* сбалансировано. Но `MemoryObject::map` (`memory/object.rs:275-277`) и `map_new_range` (`address_space.rs:71-88`) разрешают External-кусок на **любом** кадре, включая зарезервированный (битмап PMM, boot-таблицы, 64 КиБ kernel-heap). Один `revoke()`/file-mmap — и кадр, который VMM никогда не выделял, уходит в PMM. |
| **P0-3** Потеря физического смещения при расщеплении | **CONFIRMED** | `address_space.rs:214-225` (`unmap_pages` head/tail) и `:1025-1042` (`protect` head/covered/tail) копируют `*m` целиком, включая `m.pa`; потребители `address_space.rs:522` (`copy_nonoverlapping(old_pa…, 4096)`) и `:717-731` (`mmu_map_object(child.root, base, orig.pa, size, …)`) | **P0, достижим сегодня** | Сценарий уже исполняется в boot-гейте и не проверяется: `lib.rs:1509-1529` — `map_new_range(frame.addr, 3*4096, …)` (многостраничный `Present`), затем `lib.rs:1533-1540` `protect(va0+4096, 4096, RO)` → mid и tail получают `pa = frame.addr` вместо `+4096`/`+8192`. Гейт читает через VA (аппаратура верна: `arch-aarch64/src/mmu.rs:339` пишет `paddr + offset`), поэтому расхождение невидимо. Как только этот AS форкается — ребёнок получит страницу 0 по адресам `va0+4096` и `va0+8192`; COW-break по `va0+8192` скопирует данные страницы 0. Точное воспроизведение — §2.2. |
| **P0-6a** `MemoryObject` освобождает кадры при живых маппингах | **CONFIRMED (код), сегодня НЕ ДОСТИЖИМО** | `memory/object.rs:185-199` (`Drop`), `:354-371` (`revoke`) — `(*b).deallocate(pa, self.size)` без оглядки на `self.mappings`; в `unmap_pages`/`unmap_all` деаллокация только для `PhysOwnership::Anonymous` (`address_space.rs:203-212`, `:868-875`) | **P2 сегодня, P0 при появлении file-backed mmap** | Ни один живой вызывающий не роняет объект при живом маппинге: аллокаторы делают `core::mem::forget(obj)` (`object.rs:30,83,98`), heap-объект забыт (`lib.rs:682`), а `revoke()` и `MemoryObject::clone()` **вообще не имеют вызывающих** (`rg -n "revoke\(" kernel/src` → только определение `object.rs:354`). Инвариант держится дисциплиной `mem::forget`, а не конструкцией. ADR-031 §5 сейчас врёт: «VMM никогда не освобождает кадры» истинно только потому, что освобождает MRM-объект, а он забыт. |
| **P0-6b** `MemoryObject::map()` проверяет лимит слотов **после** установки маппинга | **CONFIRMED** | `memory/object.rs:275-277` вызывает `aspace.map_pages(...)`, и только потом `:279-285` ищет свободный слот и возвращает `MappingLimitReached` | **P1** | 5-й вызов `map()` на объекте с 4 занятыми слотами (`MAX_MAPPINGS = 4`, `object.rs:128`): VA сммаплен и лист в MMU есть, вызывающий получает ошибку и считает, что mapping не удался. Достижимо, как только `MemoryObject` подключат к mmap. **Регрессия против замороженной эталонной модели**: `kernel-memory-frozen/src/object.rs:132-140` проверяет слот **до** записи, и на этот случай там есть тест `mapping_limit_returns_error` (`:282`) — но крейт не член workspace (`vivanta-boot/Cargo.toml:4-32`), и его тесты CI не запускает. |
| **P0-7** `materialize`/`capacity` оставляет «призрачный лист» | **CONFIRMED** | `address_space.rs:426-477`: `mmu_map_object` на `:446` **до** `replace_slots` на `:473-475`; пред-проверки ёмкости нет (в отличие от sibling-функции `resolve_lazy_fault`, где она есть на `:363-366`) | **P1 сегодня, P0 при образе > ~64 страниц** | `sys_execve` получает `EFAULT` (`syscall/process.rs:622-628`), но: (1) пользователь читает страницу, которую ядро считает `LazyAnonymous` — дескриптор валиден, fault не возникает; (2) `unmap_all` её не освобождает, потому что `for_hardware_runs` отфильтровывает `Lazy` (`address_space.rs:243`) → **утечка кадра навсегда**; (3) `verify_domain_reverse` (`:807-832`) такой лист поймает, но на этом пути не вызывается. AS после этого **навсегда заблокирован** (`MappingSet` полон). Сегодня недостижимо: встроенный `/init` — 65 КиБ ≈ 17 страниц (`docs/audit/2026-09-25-cleanup.md:13`), ёмкость `MappingSet` = 64 (`mapping.rs:147`). |
| **P0-9** Не транзакционный учёт COW-владения | **CONFIRMED, но описан неточно** | Инкремент **до** подмены shadow: `address_space.rs:641` и `:666`; декремент — в `replace_slots` → `mapping.rs:269-271` → `mapping.rs:305-309` | **P0, достижим сегодня** | Аудит прав про порядок, но **пропустил более тяжёлую половину**: главная поломка — не «не-транзакционность», а **асимметрия инкремента и числа кусков**. `inc(pa, 2)` при `Present` считает 2 владельца (родитель+ребёнок) и 2 декремента симметричны — поэтому симметричный teardown балансируется, и именно поэтому `[E2E]`-гейт (`lib.rs:546-556`, точные значения) зелёный. Но COW-split (`address_space.rs:530-561`) превращает 1 кусок `CoWShared{pa}` в **2** куска `CoWShared{pa}` (head+tail) + 1 `Present`, при этом реестр уменьшается на 1. Дальше первое же удаление любого из оставшихся кусков обнуляет счётчик → `dec` → 0 → `faults.rs:22-28` → `deallocate(pa)` → **кадр в PMM при двух живых отображениях**. Сценарий — §2.5. |
| **P1-a** `pmm::free_frame` не различает reserved и allocated | **CONFIRMED** | `kernel/src/pmm.rs:65-72`: снимает бит и уменьшает `allocated_count`, но `reserved_count` (увеличивается только в `reserve()`, `pmm.rs:87`) не трогает | **P0 при первом освобождении зарезервированного кадра** | Два последствия. (1) **Освобождаемый reserved-кадр становится аллоцируемым** — `alloc_frame` (`pmm.rs:19-30`) отдаст битмап PMM, boot-таблицу, DTB, MMIO-пробел или kernel-heap. (2) `reserved_count` устаревает навсегда → `free_count()` (`pmm.rs:184`) систематически завышен, а единственная проверка (`verify.rs:25-28`) тавтологична и поломку не видит. Кто может освободить reserved-кадр: `cow_free_trampoline` (`vmm/faults.rs:22-28`) по пути P0-2; `AsPageTableAllocator::reclaim_page_table_frame` (`memory/object.rs:118-122`). |
| **P1-b** Внутренние root-кадры `sys_fork` не возвращаются в PMM | **CONFIRMED** | `syscall/process.rs:81` `alloc_frame()` для root; освобождается только на ветках отказа `:83, :98-103, :117-122, :134-140`; успешного пути нет. `vmm::unregister` документированно не трогает root (`address_space.rs:1108-1111`) | **P1** | Каждый `fork()` теряет 1 кадр root (4096 Б) + 16 кадров ядерного стека (64 КиБ) на каждый завершённый дочерний поток. При 512 МиБ это ~4 МБ за 64 форка — не «утечка, которую заметят», а **лимит на число процессов**. |
| **P1-c** `mapping.rs` не отклоняет перекрытия и не валидирует выравнивание | **CONFIRMED** | `mapping.rs:163-176` (`insert`) — ни проверки пересечения, ни выравнивания; `address_space.rs:106-132` (`map_pages`) — нет проверок `vaddr % 4096`, `paddr % 4096`, `size % 4096` | **P1** | (a) Два перекрывающихся `Present`-куска: `query()` (`address_space.rs:926-930`) вернёт первый по слоту, `unmap_pages` снимет весь прогон железа, один кусок станет «призраком» с неверным `pa`, `verify_hardware_consistency` (`:750-797`) сравнивает только права и пропустит. (b) Невыровненный `paddr` **молча обрезается**: `arch-aarch64/src/mmu.rs:319` `d | (phys & ADDR_MASK)` при `ADDR_MASK = 0x…F000` (`arch-aarch64/src/paging/descriptor.rs:113`). Невыровненный `vaddr` сместит `l3`-индекс (`arch-aarch64/src/mmu.rs:337`) и затрёт соседний дескриптор. Достижимо через `MemoryObject::map` и `map_new_range`; syscall-путь защищён косвенно — `unmap_pages`/`protect` строят `VaRegion` (`address_space.rs:148, 956`), но `map_pages` — нет. |
| **P1-d** `Vec` на пользовательских и fault-путях | **CONFIRMED** | 8 живых `alloc::vec::Vec` в `address_space.rs`: `:239` (`for_hardware_runs` ← `unmap_pages`+`protect`, оба — syscall), `:389` (`resolve_lazy_fault`, fault), `:450` (`materialize_with`, execve), `:530` (`resolve_cow_fault`, fault), `:588,598,599,600,604` (`duplicate_as`, fork), `:847,876` (`unmap_all`, execve) | **P1** | Ядро — 64 КиБ free-list (`lib.rs:672`, `memory/kernel_heap.rs`), OOM → паника (`#[alloc_error_handler]` в проекте отсутствует). `size_of::<Mapping>() == 56` — вычислено на хосте репликой тех же типов (`VirtRange{u64,u64}` + `u64` + `MappingFlags{u64}` + `Backing`(1) + pad + `u64` + `PhysOwnership`(8)). `MappingSet` = 64·56 + 8 = **3592 Б**, что сходится с assert-ами `address_space.rs:12-13` (AS ≤ 12 КиБ, MappingSet ≤ 5 КиБ). **INFERRED:** пик `duplicate_as` при 64 кусках — 5 `Vec` по 64·56 = **17 920 Б = 27 % кучи**; с учётом удвоения при `push` пик выше. `for_hardware_runs` тянет до 64 кортежей `(u64,u64,bool)` = 24 Б → 1536 Б. Это фиксированный потолок, а не «мелочь на будущее». |
| **P2-a** `MemoryCapability::check()` всегда `true` | **CONFIRMED, объявленный placeholder** | `memory/capability.rs:74-78` с комментарием «Placeholder — always returns true. Full enforcement deferred to V3+» | **P2 (фактически — неактивный контракт)** | **Ноль вызывающих** во всём `kernel/src` (`rg -n "\.check\(" kernel/src` → только `va_tests.rs`). Права не проверяются нигде; `MemoryObject::map` игнорирует `capability.rights`, кроме `rights_to_flags` (`object.rs:274`, `:384-393`), где биты `MemRights::map`/`share` **не переносятся** в `MappingFlags` вовсе. Не дефект кода — незакрытый контракт. |
| **P2-b** `register`/`register_child` паникуют при исчерпании пула | **CONFIRMED, и это хуже, чем в аудите** | `address_space.rs:1073-1088` и `:1090-1103`; пул `MAX_ADDRESS_SPACES = 16` (`:19`), комментарий `:16-18` прямо признаёт «Raised from 8 … Exhaustion still panics deterministically at registration» | **P1 на практике (аудит даёт P2)** | Пул уже почти исчерпан: `kernel/src/lib.rs` содержит **16** мест `vmm::register(` и **13** мест `vmm::unregister(`; как минимум 8 зарегистрированных AS никогда не снимаются (`user_as1`/`user_as2` `:885-886`, три fault-сценария `:1112`, `elf_as` `:1223`, `cow_parent`/`cow_child` `:1301`/`:1335`). **INFERRED** (точную цифру не гонял, QEMU недоступен): пик близок к 14-15 из 16. Следствие: добавление **одного** нового гейта, регистрирующего AS, валит всю загрузку. Паника убирает терминальный маркер, и `qemu-gates.sh:115` печатает `WARN: terminal marker not reached` — отказ выглядит как таймаут, а не как исчерпание пула. |

---

## 2. Root-cause analysis по подтверждённым дефектам

### 2.1 P0-2 — `duplicate_as` переклассифицирует `External` в `CoWShared`

**Механизм.** `AddressSpace::map_pages` записывает shadow-кусок с
`PhysOwnership::External` (`address_space.rs:120-126`, комментарий: «PA recorded
in the shadow (External ownership: the caller owns the physical memory)»).
`duplicate_as` обрабатывает `Backing::Present` одним безусловным ветвителем
(`:606-642`): независимо от `m.phys` он ставит
`parent_piece.phys = CoWShared { refcount: 2 }` (`:617-619`), создаёт дочерний
`cow_shared(..., 2)` (`:627-633`) и регистрирует кадр в глобальном реестре
(`cow_refcount::inc(m.pa, 2)`, `:641`). ADR-034 §2 требует обратного:
«Present(**External**) → child получает тот же PA, то же permissions, но **без
write**» — то есть read-only **Present**, а не `CoWShared`; `CoWShared`
добавляется только в ветке «Present(**Anonymous**)»
(`ADR-034-cow-semantics.md:52-54`).

**Нарушенный инвариант.** ADR-032 §4: «`External` — caller/MemoryObject владеет
PA; VMM никогда его не освобождает (alias policy of ADR-031 сохранена)»
(`ADR-032:142-144`). ADR-031 §5: «physical-frame ownership always stays with the
allocator's client … The VMM never frees physical frames»
(`ADR-031:108-112`).

**Цепочка до ущерба.** Реестр обнуляется → `cow_refcount::dec`
(`cow_refcount.rs:89-108`) вызывает `FREE_FN` → `cow_free_trampoline`
(`vmm/faults.rs:22-28`) → `(*backend).deallocate(pa, 4096)` →
`PmmBackend::deallocate` (`memory/pmm_backend.rs:44-52`) →
`PmmBitmap::free_frame` (`pmm.rs:65-72`) → бит кадра снят. С этого момента
`alloc_frame` (`pmm.rs:19-30`) может вернуть его снова.

**Сценарий (входы → неверный результат).**

1. `MemoryObject` на 1 кадр, `phys_addr = P`. `MemoryObject::map(P, 4096, aspace, …)`
   (`memory/object.rs:259-287`) → `map_pages` → shadow: `Present, pa=P,
   phys=External, object_id = obj.id`.
2. `sys_fork` (`syscall/process.rs:130`) → `duplicate_as` → родитель и ребёнок
   получают `CoWShared{P,2}`, `R(P) = 2`.
3. Родитель делает `munmap` этого диапазона → `MappingSet::remove`
   (`mapping.rs:178-191`) → `release_cow_frame` → `dec(P)` → `R(P) = 1`. Кадр
   **не** освобождён.
4. Ребёнок делает `munmap` → `dec(P)` → 0 → `deallocate(P, 4096)` → **P снова
   свободен в PMM**, хотя `MemoryObject`-владелец всё ещё считает своим.
5. Любой следующий `allocate_page` (`pmm.rs:200-204`, `lib.rs:711`, или
   `AsPageTableAllocator::alloc_page_table_frame`, `object.rs:78`) получает `P`
   и **перезаписывает кадр**. Если за это время объект успели `Drop`/`revoke`
   (`object.rs:185-199`, `:354-371`), `deallocate(P, size)` вызывается
   **второй раз** — уже по адресу, принадлежащему новому владельцу. Двойное
   освобождение чужого кадра: тихая порча таблиц страниц, страниц ELF или
   живого объекта.

**Почему не поймано сейчас.** Boot-гейт COW (`lib.rs:1302-1355`) — единственное
место в дереве, где `External`-кусок форкается; там `P` взят из
`pmm.alloc_frame()` (`lib.rs:1302`), поэтому шаги 4-5 случайно сходятся. Сам
гейт после break AS не снимает (нет `unregister` после `lib.rs:1356`), так что
`R(P) = 1` просто остаётся висеть.

**Компункт с P0-6a.** Если в шаге 1 объект дополнительно `revoke()`-нуть, шаги
3-5 дают **двойное освобождение одного и того же кадра** (сначала COW-реестр,
затем `Drop`). Это самая опасная комбинация двух пунктов бэклога, и она
полностью определяется двумя файлами.

### 2.2 P0-3 — расщепление куска теряет физическое смещение

**Механизм.** `Mapping.pa` документирован как «Physical base for Present
pieces» (`mapping.rs:56`). Куски бывают многостраничными: `map_pages` принимает
произвольный `size` (`address_space.rs:106-114`), а `map_new_range` в boot-гейте
передаёт `3*4096` (`lib.rs:1514-1515`). При расщеплении `unmap_pages`
(`:214-225`) и `protect` (`:1025-1042`) head/tail/covered строятся как
`let mut piece = *m; piece.virt_range = …` — `pa` копируется **без** пересчёта
`pa += (new_base - old_base)`. Дальше:

- `resolve_cow_fault` читает `old_pa = m.pa` (`:497`) и копирует
  `copy_nonoverlapping(old_pa, new_frame, 4096)` (`:522`) — для страницы, не
  являющейся первой в куске, читается **не та** физическая страница.
- `duplicate_as` для дочернего HW зовёт `mmu_map_object(child.root,
  orig.virt_range.base, orig.pa, orig.virt_range.size, …)` (`:722-729`) —
  `mmu_map_object` мапит `paddr + offset` на страницу
  (`arch-aarch64/src/mmu.rs:339`), то есть хвост куска отображается на
  `pa, pa+4096, …` вместо `pa + offset_tail, …`.
- `unmap_all` (`:868-875`) и `unmap_pages` (`:203-212`) освобождают
  `deallocate(m.pa, m.virt_range.size)` — для хвостового куска это
  `deallocate(pa, tail_size)`, то есть освобождается **не тот** диапазон.

**Нарушенный инвариант.** ADR-032 §1: «Shadow pieces are kept exact by splitting
… so piece granularity always equals hardware granularity and INV-VM-001 stays
mechanical» (`ADR-032:54-58`). ADR-032 §0 (ядро): `MappingSet` — «single
authoritative registry of user virtual memory», page tables — «a materialization
of `MappingSet`, never an independent source of state» (`:15-19`). Расхождение
`pa` — прямое нарушение этого утверждения.

**Сценарий (входы → неверный результат).** Ровно то, что уже делает boot-гейт,
плюс один `fork`:

```
1. pmm.alloc_contiguous(3) → P0, P1, P2                    (lib.rs:1509)
   записать: P0[0]=0x1111, P1[0]=0x2222, P2[0]=0x3333        (lib.rs:1527-1529)
2. map_new_range(P0, 3*4096, RW) → VA0                      → shadow: Present{VA0..VA0+12K, pa=P0}
3. protect(VA0+4096, 4096, RO)                              (lib.rs:1533)
   → head  Present{VA0,    +4K, pa=P0}   корректно
   → mid   Present{VA0+4K, +4K, pa=P0}   ДОЛЖНО быть P1
   → tail  Present{VA0+8K, +4K, pa=P0}   ДОЛЖНО быть P2
   (аппаратура верна: mmu_protect переписывает права, PA не трогает)
4. fork() → duplicate_as                                    (address_space.rs:606-731)
   child HW: mmu_map_object(child, VA0+4K, pa=P0, 4K) → child читает 0x1111
   child HW: mmu_map_object(child, VA0+8K, pa=P0, 4K) → child читает 0x1111
   ОЖИДАЛОСЬ 0x2222 и 0x3333. ПОЛУЧЕНО 0x1111 и 0x1111.
5. child пишет в VA0+8K → COW break
   copy_nonoverlapping(P0, new_frame, 4096)                (address_space.rs:522)
   ОЖИДАЛОСЬ в new_frame 0x3333. ПОЛУЧЕНО 0x1111.
6. unmap(VA0+8K) в родителе → deallocate(P0, 4096)
   → P0 возвращён в PMM, хотя P1 и P2 того же прогона ещё живы и позже будут
     освобождены как deallocate(P0, …) → повторное освобождение диапазона,
     начинающегося с P0.
```

**Почему QEMU-гейт зелёный.** `verify_hardware_consistency`
(`address_space.rs:750-797`) сравнивает **только биты прав** (`desc & expected`,
`:766-774`) и наличие валидного листа; поле PA дескриптора не читается ни разу.
`verify_domain_reverse` (`:807-832`) смотрит только на бит 0.
`mmu_leaf_descriptor` (`arch-aarch64/src/mmu.rs:498-501`) возвращает полный
дескриптор, включая `ADDR_MASK`-поле, — данные для проверки есть, проверки нет.
Плюс гейт читает через VA (`lib.rs:1542`, `:1568`), а аппаратура корректна,
поэтому расхождение принципиально невидимо без форка.

### 2.3 P0-6 — `MemoryObject` освобождает кадры при живых маппингах

**Механизм (6a).** `MemoryObject` владеет кадрами сам (`object.rs:165-183`:
«Owns its physical frames: on `drop` (or `revoke`) the underlying backend's
`deallocate` is invoked»). `Drop` (`:185-199`) и `revoke` (`:354-371`) вызывают
`deallocate(pa, size)` безусловно и **затем обнуляют `self.mappings`**
(`:368-370`) — объект «забывает» маппинги, но `AddressSpace.mappings` и
дескрипторы MMU о них не знают. `AddressSpace` никогда не обращается к
`MemoryObject` (в `Mapping` лежит только `object_id: u64`, `mapping.rs:53`),
поэтому обратной связи нет.

**Нарушенный инвариант.** ADR-031 §5 (`:106-115`): «unmap only removes
translations and reclaims *table* frames. Therefore "unmap(A) frees P while B
still maps P" is structurally impossible». Здесь структурной невозможности нет
ровно настолько, насколько владение вынесено из VMM в объект.

**Механизм (6b).** `map()` (`:259-287`) сначала мапит (`:275-277`), потом ищет
слот (`:279-285`). Никакого отката. `VmmError` при этом теряется: любой отказ
`map_pages` (например `MappingTableFull`) превращается в
`ObjectError::MappingLimitReached` (`:277`) — неверная диагностика **плюс**
уже установленный маппинг.

**Сценарий (6b).** `MAX_MAPPINGS = 4` (`object.rs:128`). Четыре `map()`
проходят. Пятое: `aspace.map_pages` вставляет кусок и ставит лист в MMU (`:275`),
затем цикл `:279-285` не находит слот → `Err(MappingLimitReached)`. Вызывающий
решает, что маппинга нет. Повторить 1000 раз → 1000 живых отображений и 1000
«неудачных» вызовов; `MappingSet` переполняется за 64, после чего `map_pages`
начинает падать на `insert` (`:124-126`) — и объект больше не может снять
**уже установленные** куски через `unmap` (у него нет на них слотов). Кадры
текут, куски — нет.

**Сценарий (6a).** `map(phys, 4096, aspace)` (это реально делается в загрузке,
`lib.rs:941-943`) → объект выходит из области видимости → `Drop` →
`deallocate(phys, 4096)` (`object.rs:189-195`). Дескриптор в MMU остаётся
валидным и указывает на теперь свободный кадр. Следующая `allocate_page`
отдаёт его, и `mmu_map_object`/`copy_nonoverlapping` пишут по нему — мы пишем
в чужую страницу таблиц. ADR-031 §5-формулировка сейчас верна только по
счастливому обстоятельству: в загрузке объект успевает `unmap` до конца блока
(`lib.rs:958-959`), а аллокаторы страниц делают `mem::forget`.

### 2.4 P0-7 — «призрачный лист» в `materialize_with`

**Механизм.** ADR-032 §5 и «hard rule #6» требуют порядка validate →
allocate → hardware → **shadow последним**. `resolve_lazy_fault` этому
соответствует: пред-проверка ёмкости `address_space.rs:363-366` («Without this,
a nearly full MappingSet turns a user fault into a kernel panic»), затем
allocate, `mmu_map_object` (`:377-379`), затем `replace_slots` с `.expect`
(`:415-418`). `materialize_with` (`:426-477`) — прямой двойник, вызываемый из
ELF-загрузчика (`exec/mod.rs:73`) — **пред-проверки не имеет**: `mmu_map_object`
на `:446`, `replace_slots(...)?` на `:473-475`. Механизм дефекта — буквально
«забыли скопировать пре-чекст из соседней функции».

**Нарушенный инвариант.** ADR-032 §0 / INV-VM-001 (`:15-19`, `:28-30`):
«Present ⇔ valid leaf …; Lazy ⇔ no leaf». После отказа остаётся `Lazy` (в
shadow) + valid leaf (в MMU) — ровно тот «призрачный лист», который INV-VM-001
запрещает и который `verify_domain_reverse` (`:807-832`, esp. `:826-828`)
существует чтобы ловить.

**Сценарий (входы → неверный результат).**

1. `sys_execve` (`syscall/process.rs:612-628`) вызывает `unmap_all`, затем
   `load_elf`.
2. `load_elf` (`exec/mod.rs:47`) делает `reserve_at` на сегмент, затем
   `materialize_with` постранично. Пусть сегмент даёт 70 файловых страниц.
3. На 65-й странице `MappingSet` полон. `materialize_with` уже вызвал
   `mmu_map_object` (`:446`) и записал в свежий кадр данные ELF.
4. `replace_slots` → `Err(())` (`mapping.rs:262-264`) → `?` →
   `VmmError::MappingTableFull` → `load_elf` → `sys_execve` возвращает `EFAULT`.
5. Что теперь: страница 65 валидно отображена, но её нет ни в одном
   shadow-куске. Пользователь читает её без fault. `unmap_all` (`:837-888`)
   её не видит: `for_hardware_runs` отбрасывает `LazyAnonymous` (`:243`) →
   `mmu_unmap` не зовётся → дескриптор переживает teardown; кадр никогда не
   возвращается. `unregister` (`:1113-1140`) требует `mappings.len() == 0` —
   это выполнено, AS снимается, а ghost-leaf остаётся в его корне.
6. Хуже: `MappingSet` полон ⇒ **любая** последующая `materialize_with`,
   `resolve_lazy_fault` (`:363`) и `resolve_cow_fault` (`:510`) в этом AS будут
   отказывать. AS «окирпичен» до конца жизни процесса.

### 2.5 P0-9 — учёт COW-владения не транзакционный **и** асимметричен

Самый недооценённый пункт. Аудит верно указывает на порядок «инкремент до
подмены shadow, декремент после», но главная поломка — в другом.

**Механизм.** Два независимых учёта: (а) `PhysOwnership::CoWShared{refcount}` в
shadow-куске, (б) глобальный реестр `cow_refcount` (`cow_refcount.rs:49`).
Они обязаны совпадать: `inc` вызывается **только** в `duplicate_as`
(`address_space.rs:641`, `:666`), `dec` — **только** из
`MappingSet::remove`/`replace_slots` через `release_cow_frame`
(`mapping.rs:305-309`, `:181-184`, `:269-271`).

Нужный инвариант: **`R(pa)` == число живых shadow-кусков с `pa: pa`, и обновление
парное**. Нарушен дважды.

**(i) Асимметрия при COW-split.** `resolve_cow_fault`
(`address_space.rs:482-563`) заменяет **один** кусок `CoWShared{pa, n}` на
**два** `CoWShared{pa, n}` (head `:532-539`, tail `:548-555`) плюс один
`Present(Anonymous, new_frame)` (`:540-546`). Число кусков, указывающих на `pa`,
растёт с 1 до 2. `replace_slots` (`:558-561`) декрементирует реестр **на 1**
(за счёт удаляемого куска) — `release_cow_frame` вызывается только для
`affected_values` (`mapping.rs:269-271`), новые куски в реестр **не вносятся**.
Итого: `R(pa)` уменьшился на 1, а реальных владельцев стало больше на 1.
Расхождение **накапливается** с каждым break-ом.

Конкретно, `P` с `R(P) = 2` (родитель + ребёнок, куски по 3 страницы):

- ребёнок пишет в `page0` → куски на `P`: родитель(1) + ребёнок head/tail(2) =
  **3**; `R(P) = 1`.
- кто угодно размапливает любой из трёх → `dec(P)` → **0** →
  `faults.rs:22-28` → `deallocate(P, 4096)` → **P в PMM, два отображения ещё
  живы**.
- следующий `allocate_page` отдаёт P; `copy_nonoverlapping(old_pa, …)` в
  `resolve_cow_fault` (`:522`) читает/пишет по чужому кадру, `mmu_map_object`
  переустанавливает лист на переиспользованный кадр.

**(ii) Не-транзакционность форка.** `inc(m.pa, 2)` (`:641`) / `inc(m.pa, 1)`
(`:666`) выполняются **в цикле построения** (`:606-672`), то есть до
пред-проверки ёмкости (`:676-682`), до программирования HW (`:694-731`) и до
коммита shadow (`:734-742`). Комментарий `:674-675` («All failures occur before
the HW edit») — **неверно**: реестр уже мутирован.

`replace_slots` на `:735` не может вернуть ошибку: внешняя проверка
(`:679-682`) использует `self.mappings.len()` = **high-water индекс слота**
(`mapping.rs:223-225` → `count`, см. `:169-170`, `:187-189`), то есть
*завышает* число живых кусков, а `replace_slots` внутри использует точный
`count_minus_affected` (`mapping.rs:289-297`). Внешняя проверка, следовательно,
консервативна и `Err` недостижим — **CONFIRMED путём чтения арифметики,
INFERRED на исполнении**. Значит единственный достижимый отказ на этом пути —
**паника**: `mmu_protect` (`:700-710`) и `mmu_map_object` (`:717-731`) вызывают
`alloc_page_table_frame` (`arch-aarch64/src/mmu.rs:348`), который в
`AsPageTableAllocator` (`object.rs:78`) делает
`.expect("alloc_page_table_frame: OOM")` и паникует. В этот момент: реестр
завышен, HW родителя уже RO, HW ребёнка частично построен, shadow не тронут.
Ядро всё равно паникует, поэтому материальный ущерб нулевой — но формулировка
«транзакционный» в коде и в аудите не соответствует реальности и будет
ложно-успокаивать следующего читателя.

**(iii) Что именно ловит существующий гейт.** `[E2E]` (`lib.rs:497-558`)
сравнивает **точные** значения (`:546`, `:547-551`, `:552-556`) для
`pmm.free_count()`, `vmm::tables::total()`, `vmm::cow_refcount::total()`. Он
зелёный именно потому, что сценарий **симметричен**: `inc(P,2)` → два `dec(P)`
(по одному с каждой стороны) → P освобождён ровно один раз → баланс. Ни одного
COW-break в нём не происходит (только `load_elf`, `duplicate_as`, `unmap_all`,
`unregister`). Стоит добавить один break до teardown — и `assert_eq!(pmm.free_count(),
free_pre)` на `:546` упадёт. Это самая дешёвая из всех возможных регрессий,
и она уже записана в коде: нужен только сценарий.

### 2.6 P1-a — `free_frame` не различает reserved и allocated

**Механизм.** Один бит на кадр (`pmm.rs:232-244`), три счётчика:
`allocated_count` (`pmm.rs:23`; инкремент в `alloc_frame`/`alloc_contiguous`,
декремент в `free_frame`), `reserved_count` (инкремент **только** в `reserve()`,
`pmm.rs:87`), `free_count() = total - allocated - reserved` (`pmm.rs:184`).
`free_frame` (`pmm.rs:65-72`) смотрит только `self.test(idx)` — «бит установлен,
значит можно снять» — и не различает, кто его поставил.

**Сценарий (входы → неверный результат).**

1. `pmm.reserve(bitmap_start, bitmap_pages*0x1000)` (`pmm.rs:106`/`:148`) —
   битмап PMM помечен; `reserved_count += n`.
2. `free_page(bitmap_start)` — любой путь, отдающий адрес битмапа в
   `PmmBackend::deallocate` (`memory/pmm_backend.rs:44-52`). Реальные такие
   пути: `cow_free_trampoline` (`vmm/faults.rs:22-28`) по пути P0-2, если
   `duplicate_as` сделал `CoWShared` из `External`-куска на зарезервированном
   кадре; `AsPageTableAllocator::reclaim_page_table_frame` (`object.rs:118-122`).
3. `test(idx) == true` → бит снят, `allocated_count` уменьшен (он и не был
   увеличен → `saturating_sub` гасит в ноль), `reserved_count` **не** уменьшен.
4. Теперь `alloc_frame` (`pmm.rs:19-30`) **вернёт битмап PMM** в качестве
   пользовательской страницы. Следующая запись в PMM (`set`, `pmm.rs:237-244`)
   или чтение (`test`) уничтожает учёт всей физической памяти.
5. Параллельно `free_count()` завышен на 1 навсегда → `verify_pmm` не заметит
   (тавтология, §0).

**Дополнительно.** `PmmBitmap::init` (`pmm.rs:159-172`) делает
`core::ptr::write_bytes(bitmap_start, 0, byte_len)` **поверх** самих frames
области — битмап лежит в начале `region`. `reserve` вызывается *после*
`write_bytes`, что корректно только потому, что `reserve` считает собственный
битмап уже занятым. Если кто-то освободит эти кадры, следующая `reserve`
другого диапазона или `alloc_frame` перезапишет битмап.

### 2.7 P1-c — нет валидации перекрытий и выравнивания

**Механизм (перекрытия).** `MappingSet::insert` (`mapping.rs:163-176`) — чистый
«первый свободный слот», никакой проверки диапазонов. В `AddressSpace`
перекрытие может возникнуть только через прямой `map_pages` (не через
`map_new_range`, где VA берётся из `VaAllocator`, а `reserve` в `va.rs:143-149`
гарантирует покрытие свободным интервалом). Прямые вызывающие:
`MemoryObject::map` (`object.rs:275`) и boot-гейты (`lib.rs:982`, `:1317`,
`:1623`). Из них `lib.rs:982` и `:1623` мапят идентичный VA, уже покрытый
2 МиБ identity-блоком, — но **shadow** о нём не знает, так что «железное»
перекрытие допустимо и обязано остаться допустимым (ADR-031 §5 явно разрешает
виртуальные алиасы, а boot целиком построен на identity-блоках). Значит
проверка должна быть **только по shadow-кускам** — это важное ограничение для
дизайна (§3.7).

**Механизм (выравнивание).** Ни `map_pages` (`address_space.rs:106-132`), ни
`MemoryObject::map` (`object.rs:259-287`) не проверяют `vaddr % 4096`,
`paddr % 4096`, `size % 4096`. `mmu_map_object` это не спасает:
`flags_to_desc_bits` (`arch-aarch64/src/mmu.rs:319`) делает
`d | (phys & ADDR_MASK)` с `ADDR_MASK = 0x0000_FFFF_FFFF_F000`
(`arch-aarch64/src/paging/descriptor.rs:113`) → **младшие 12 бит `paddr` молча
отбрасываются**, пользователь получает страницу по адресу, округлённому вниз.
Невыровненный `vaddr` сместит `l3`-индекс (`arch-aarch64/src/mmu.rs:337`,
`walk_to_l3(root, va)`) и затрёт дескриптор **соседней** страницы.
`VirtRange::end()` (`mapping.rs:16-18`) — сложение без проверки переполнения.

**Сценарий.** `map_pages(0x1000_1001, 0x8000_0001, 4096, RW, …)` →
дескриптор ставится на `0x1000_1000 → 0x8000_0000`: писатель в `0x1000_1001`
получит доступ к `0x8000_0000`, а страница `0x1000_0000` потеряет отображение
(если была). Для shadow это выглядит как корректный кусок;
`verify_hardware_consistency` права проверит и пройдёт, PA — нет (§2.2).

### 2.8 P1-d — `Vec` на пользовательских и fault-путях

**Механизм.** `KernelHeap` — free-list на 64 КиБ, выделенных из MRM
(`lib.rs:672-678`, `memory/kernel_heap.rs:23-33`). `alloc`/`dealloc` берут
`crate::interrupts_guard()` и спин-локу (`:242-266`). OOM → `alloc` возвращает
null → `handle_alloc_error` по умолчанию → **паника**;
`#[alloc_error_handler]` в проекте отсутствует (`rg -n "alloc_error_handler" .`
→ пусто). Список живых аллокаций на путях — 8 `Vec` в `address_space.rs`
(строки в §1, P1-d).

**Бюджет (INFERRED, вычислено).** `size_of::<Mapping>() == 56` (хост-реплика
тех же типов; сходится с `assert!(size_of::<MappingSet>() <= 5*1024)` на
`address_space.rs:13`, где 3592 Б). `duplicate_as` держит одновременно пять
`Vec<Mapping>` (`:588`, `:598`, `:599`, `:600`, `:604`) — при 64 кусках это
**17 920 Б = 27 % кучи**; с удвоением ёмкости при `push` пик выше. Плюс
`pieces` в трёх других функциях.

**Про важную поправку к собственному анализу.** Спин-локу кучи **не угрожает**
реентерабельность из таймерного IRQ: `check_sleeping_threads`
(`scheduler/mod.rs:204-216`) собирает `Vec<ThreadId>` и вызывается из
`schedule_tick` (`:424-427`) → `maybe_reschedule` (`:429-435`), но
`disable_interrupts` **действительно маскирует IRQ** (`DAIFSet #2` = бит I;
см. §0), поэтому таймерный IRQ не входит в `alloc_locked`. Реальные риски — только
два: паника на OOM и предсказуемый потолок 17-27 КиБ на `fork()`.

---

## 3. Дизайн фиксов

### 3.0 Единая модель владения (проектируется один раз)

Сегодня владение выражено **тремя независимыми механизмами**, которые могут
разойтись, и именно их расхождение даёт P0-2, P0-6a, P0-7, P0-9 и P1-a:

| Слой | Механизм | Где живёт | Кто обновляет |
|---|---|---|---|
| кадр | один бит в битмапе | `pmm.rs:232-244` | `alloc_frame`/`free_frame`/`reserve` |
| shadow-кусок | `PhysOwnership` | `mapping.rs:38-48` | `map_pages`, `duplicate_as`, пять расщеплений |
| меж-AS | реестр `cow_refcount` | `cow_refcount.rs:49` | **только** `duplicate_as` (inc) и `MappingSet::remove`/`replace_slots` (dec) |

**Предлагаемая модель: один владелец на кадр, а не счётчик.** Ключевая идея —
разделить «**кто** имеет право вернуть кадр» (идентичность) и «**сколько**
ссылок» (счётчик), и запретить переход из первой категории во вторую.

```rust
// kernel/src/memory/frame_owner.rs (новый файл, ~90 строк)

/// Кто имеет право вернуть кадр в PMM. Категория владельца никогда не
/// меняется после выделения; меняется только счётчик ссылок внутри `Vm`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FrameOwner {
    /// Кадр принадлежит платформе: битмап PMM, boot-таблицы, DTB, MMIO-пробелы.
    /// Ссылок на него не бывает; VMM его никогда не освобождает.
    Platform,
    /// Кадр принадлежит MemoryObject (ADR-032 §4 External).
    /// `id` = `MemoryObjectId`, чтобы `revoke()` мог проверить нулевое число
    /// ссылок перед освобождением.
    Object(u64),
    /// Кадр выделен VM-слоем. `refs` — число живых shadow-кусков, ссылающихся
    /// на кадр. `refs == 1` означает «владеет ровно этот кусок» (бывший
    /// `PhysOwnership::Anonymous`); `refs > 1` — COW (бывший `CoWShared`).
    Vm { refs: u32 },
}

pub fn owner_of(pa: u64) -> FrameOwner;   // не найден => Platform
pub fn set_owner(pa: u64, o: FrameOwner) -> Result<(), OwnerError>; // refalloc
pub fn acquire(pa: u64) -> Result<(), OwnerError>;  // refs += 1
pub fn release(pa: u64) -> bool;          // true => «последний, верни в PMM»
pub fn refs_of(pa: u64) -> u32;
pub fn refs_for_object(id: u64) -> u32;
pub fn sweep() -> usize;                  // сколько Platform-кадров упомянуто
```

**Почему это правильная форма, а не «ещё один счётчик»:**

1. `Platform` делает **невозможной** саму ошибку P1-a: `release()` на кадре с
   владельцем `Platform` обязан вернуть `false` (не освобождать) и зафиксироваться
   в `sweep()`. «Reserved» перестаёт быть свойством битмапа и становится
   свойством реестра — то есть `free_frame` получает право **отличать** reserved
   от allocated не по эвристике, а по факту.
2. `Vm { refs }` — **единственное** место, где живёт число, и обновляется оно
   ровно в двух точках, обе из которых уже существуют (`duplicate_as` и
   `MappingSet`). Сейчас этих точек четыре (`inc` в двух ветках, `dec` в двух
   функциях `MappingSet`), и P0-9 — прямое следствие того, что `release`
   вызывается для `affected_values`, а не для `pieces`.
3. `Object(id)` делает P0-6a **конструктивно невозможным**: `MemoryObject::revoke`
   может требовать `refs_for_object(id) == 0`, а `refs` на нуле ровно тогда,
   когда в `MappingSet` не осталось кусков с этим `object_id`. «Сначала сними
   маппинги, потом освободи кадры» перестаёт быть дисциплиной и становится
   следствием.
4. Он **заменяет** `PhysOwnership` в shadow, а не добавляется к нему:
   `Mapping.phys: PhysOwnership` → `Mapping.phys: FrameOwner`. Размер `Mapping`
   не растёт (`FrameOwner` = 8 байт, `PhysOwnership` = 8 байт).
5. Переход `External → CoWShared` становится **непредставимым в типах**:
   `duplicate_as` физически не может сделать `Object(_) → Vm { .. }`, потому что
   `FrameOwner` не имеет такого перехода. Это ровно тот случай, где «нельзя
   написать» сильнее «не надо писать».

**Транзакционность.** Операции, меняющие и реестр, и shadow, обязаны идти через
один helper, а не две независимые правки:

```rust
// mapping.rs — единственное место, где меняется счётчик ссылок
pub fn replace_slots(&mut self, affected: &[Mapping], pieces: &[Mapping]) -> Result<(), ()> {
    // … проверка ёмкости ДО любой мутации (уже есть, mapping.rs:261-264) …
    for a in affected { frame_owner::release(a.pa); }   // ровно один dec на кусок
    let mut next = Self::new();
    for (_, m) in self.iter_with_slots() {
        if !affected.iter().any(|a| a.virt_range.base == m.virt_range.base
                              && a.virt_range.size == m.virt_range.size) {
            next.insert(*m);
        }
    }
    for p in pieces {
        if let FrameOwner::Vm { .. } = p.phys { frame_owner::acquire(p.pa).map_err(|_| ())?; }
        next.insert(*p);
    }
    *self = next;
    Ok(())
}
```

Ключевое: **`inc` переезжает из `duplicate_as` в `replace_slots`**. Тогда
`duplicate_as` вообще не трогает реестр, транзакционность становится
структурной (состояние реестра меняется ровно в одной функции, рядом с проверкой
ёмкости), и исчезает целый класс расхождений. Симметрично `resolve_lazy_fault`,
`materialize_with` и `resolve_cow_fault` получают `acquire` на каждый свой
`Vm { refs }`-кусок — то есть приватное владение это просто `refs == 1`.

`MappingSet::remove` (`mapping.rs:178-191`) остаётся вторым (и последним)
вызывающим `release`, и если он возвращает `true`, вызывает `deallocate` через
`faults::anonymous_backend()`. Это **убирает третью копию логики владения**
(сегодня она продублирована в `unmap_pages:203-212` и `unmap_all:868-875`).

**Инварианты, которые надо зафиксировать в коде:**

- **I-OWN-1** `refs(pa) == |{ m ∈ все AS : m.pa == pa ∧ m.phys == Vm }|`.
- **I-OWN-2** у `Platform`-кадра `refs == 0`; `free_frame` на нём — no-op + `sweep()`.
- **I-OWN-3** `m.pa + (va - m.virt_range.base) == desc & ADDR_MASK` (проверяемо в
  `verify_hardware_consistency`, §4.1).
- **I-OWN-4** `va % 4096 == 0 ∧ pa % 4096 == 0 ∧ size % 4096 == 0 ∧ size > 0` для
  любого установленного куска.
- **I-OWN-5** куски попарно не пересекаются по `virt_range`.
- **I-OWN-6** `refs_for_object(id) == 0` для каждого `MemoryObject` в
  `Allocated`-состоянии.

### 3.1 P0-2 + P0-9 + P0-6a — владение кадрами (одна работа, три дефекта)

**Файлы:** `kernel/src/memory/frame_owner.rs` (новый), `kernel/src/vmm/mapping.rs`,
`kernel/src/vmm/cow_refcount.rs` (сливается в `frame_owner`, отдельный модуль
уходит), `kernel/src/vmm/address_space.rs`, `kernel/src/memory/object.rs`,
`kernel/src/vmm/faults.rs`.

1. `Mapping.phys: PhysOwnership` → `Mapping.phys: FrameOwner` (`mapping.rs:38-48`,
   `:58`). `Backing` не трогаем: он про MMU, не про владение.
2. `duplicate_as` (`address_space.rs:606-671`) переписан по ADR-034 §2 дословно,
   тремя отдельными ветками:

```rust
Backing::Present => match m.phys {
    // ADR-034 §2.1 — Present(External): child получает тот же PA и permissions
    // минус write. Это НЕ COW: копии нет и не будет, оба просто читают.
    FrameOwner::Object(_) | FrameOwner::Platform => {
        parent_pieces.push(*m);                       // phys НЕ меняется
        child_pieces.push(Mapping::present(
            m.virt_range, m.object_id, m.permissions, m.pa, m.phys,
        ));
        // ЖЕЛЕЗО: parent RO через mmu_protect (:694-710),
        //         child  RO через mmu_map_object (:717-731)
    }
    // ADR-034 §2.2 — Present(Anonymous): CoWShared{refcount += 1} у обоих
    FrameOwner::Vm { refs } => {
        let n = refs + 1;
        parent_pieces.push(Mapping::cow_shared(m.virt_range, m.object_id,
                                               m.permissions, m.pa, n));
        child_pieces.push(Mapping::cow_shared(m.virt_range, m.object_id,
                                              m.permissions, m.pa, n));
        // ++2 регистрирует replace_slots, НЕ эта функция
    }
},
Backing::CoW => match m.phys {
    FrameOwner::Vm { refs } => { /* вложенный форк: n = refs + 1 у обоих */ }
    // Ветка CoW + не-Vm владелец непредставима в типах — это и есть
    // структурная защита от P0-2.
    _ => unreachable!(),
},
Backing::LazyAnonymous | Backing::Reserved => child_pieces.push(*m),
```

   Реестр в `duplicate_as` больше не трогается **вообще**. Комментарий
   `:674-675` («All failures occur before the HW edit») становится правдой, и
   `shared_orig` (`:604`) можно удалить — hardware-циклы итерируют
   `parent_new`/`child_pieces` напрямую.
3. `PhysOwnership::Anonymous` исчезает как отдельный смысл: он становится
   `FrameOwner::Vm { refs: 1 }`. Специальная деаллокация в `unmap_pages`
   (`:203-212`) и `unmap_all` (`:868-875`) удаляется — освобождение делает
   `MappingSet::remove`/`replace_slots` через `frame_owner::release`.
4. `MemoryObject` (P0-6a). `Drop`/`revoke` (`object.rs:185-199`, `:354-371`) больше
   не зовут `deallocate` напрямую:

```rust
// object.rs
pub fn revoke(&mut self) -> Result<(), ObjectError> {
    if self.mappings.iter().any(|m| m.is_some()) { return Err(ObjectError::StillMapped); }
    if frame_owner::refs_for_object(self.id) > 0 { return Err(ObjectError::StillMapped); }
    if self.owns_storage {
        if let Some(b) = self.backend {
            if let Some(pa) = self.phys_addr { unsafe { (*b).deallocate(pa, self.size); } }
        }
    }
    self.state = MemoryObjectState::Revoked;
    self.owns_storage = false;
    self.phys_addr = None;
    Ok(())
}
```

   `Drop` вызывает `revoke()` и **игнорирует** результат: если маппинги живы —
   кадры **не** освобождаются. Это правильная асимметрия — утечка детектируема,
   UAF нет. `MemoryObject::clone` (`object.rs:326-340`) перестаёт быть
   `owns_storage: false` с комментарием «No refcount yet (deferred)» (`:335-336`):
   клон регистрирует ссылку `frame_owner` (или получает `Object(orig_id)` +
   `acquire`), и `Drop` оригинала тогда безопасен.
5. `MemoryCapability::check` (P2-a) — реализовать заявленное:

```rust
pub fn check(&self, required: MemRights) -> bool {
    (!required.read    || self.rights.read)
      && (!required.write   || self.rights.write)
      && (!required.execute || self.rights.execute)
      && (!required.map     || self.rights.map)
      && (!required.share   || self.rights.share)
}
```

   и **вызвать** в `MemoryObject::map` до `map_pages`, иначе это снова
   декоративный код. Дополнительно: `rights_to_flags` (`object.rs:384-393`)
   молча роняет `map`/`share` — у `MappingFlags` таких битов нет; это ожидаемо,
   но должно быть сказано явно, иначе следующий читатель решит, что права
   теряются по ошибке.

**Восстановленный инвариант:** ADR-031 §5 + ADR-032 §4. VMM не освобождает кадры
`Object`/`Platform`; освобождение происходит ровно тогда, когда реестр видит ноль
ссылок, а реестр видит ноль ссылок ровно тогда, когда ни один shadow-кусок не
указывает на кадр.

**Объём: large.** 5 файлов + новый, меняет смысл публичного поля в `Mapping`,
требует пересчёта всех мест с `PhysOwnership` (`rg -n PhysOwnership kernel/src` —
12 вхождений в 3 файлах). Оправдано: закрывает сразу 4 пункта бэклога и даёт
инфраструктуру для `verify_hardware_consistency` (§4.1) и точных гейтов (§5).

### 3.2 P0-3 — смещение PA при расщеплении

**Файлы:** `kernel/src/vmm/address_space.rs` (расщепления), `kernel/src/vmm/mapping.rs`.

Сегодня логика head/covered/tail продублирована **пять** раз: `resolve_lazy_fault`
(`:389-410`), `materialize_with` (`:450-471`), `resolve_cow_fault` (`:530-555`) и
ещё два head/tail (`unmap_pages:214-225`, `protect:1025-1042`) — пять копий,
две из которых забывают про `pa`. Правильная форма — вынести в одно место:

```rust
// mapping.rs
impl Mapping {
    /// Вырезать `[cut_lo, cut_hi)` из куска, вернув
    /// `[head][covered][tail]` с **пересчитанным** `pa` в каждом куске.
    /// Единственное место, где живёт правило
    /// `piece.pa = self.pa + (piece.base - self.base)`.
    pub fn split(&self, cut_lo: u64, cut_hi: u64)
        -> (Option<Mapping>, Option<Mapping>, Option<Mapping>);

    /// Физический адрес, подложенный под `va` внутри этого куска.
    /// `None` для Lazy/Reserved (нет образа в MMU).
    pub const fn pa_of(&self, va: u64) -> Option<u64> {
        match self.backing {
            Backing::Present | Backing::CoW => Some(self.pa + (va - self.virt_range.base)),
            Backing::LazyAnonymous | Backing::Reserved => None,
        }
    }
}
```

`split` применяется в `unmap_pages`, `protect`, `resolve_lazy_fault`,
`materialize_with`, `resolve_cow_fault` — везде, где сегодня стоит
`let mut piece = *m;`.

**Страховочный `debug_assert`** (то, чего не хватает всему предыдущему коду):
после сборки кусков в каждой из пяти функций

```rust
debug_assert!(pieces.iter().all(|p| p.pa_of(p.virt_range.base) == Some(p.pa)));
```

Для `pa_of(base)` это тождественно `pa`, поэтому как проверка куска бесполезно —
**проверять надо железо**, см. §4.1 (`I-OWN-3`). Ценность `debug_assert` в
другом: он ловит неправильный rebase в момент сборки, а не в момент форка.

**Объём: medium.** Один файл на ~40 строк + замена пяти мест. Риск низкий:
поведение «всё остальное» не меняется и проверяется `verify_hardware_consistency`
после правки.

### 3.3 P0-6b + P2-a — порядок в `MemoryObject::map`

**Файлы:** `kernel/src/memory/object.rs`, `kernel/src/memory/capability.rs`.

```rust
pub fn map(&mut self, vaddr: u64, size: u64, aspace: &mut AddressSpace,
           alloc: &mut dyn PageTableAllocator) -> Result<usize, ObjectError> {
    if self.is_revoked() { return Err(ObjectError::Revoked); }
    if self.state == MemoryObjectState::Created { return Err(ObjectError::NoStorage); }
    if !self.capability.check(MemRights { read: true, map: true, ..MemRights::EMPTY }) {
        return Err(ObjectError::NoRights);
    }
    let Some(phys) = self.phys_addr else { return Err(ObjectError::NoStorage) };
    // 1. Ёмкость ПЕРЕД любой мутацией (ADR-032 §5 «validate первым»)
    let slot = self.mappings.iter().position(|m| m.is_none())
        .ok_or(ObjectError::MappingLimitReached)?;
    // 2. Выравнивание до map_pages (ADR-031 §4; today: nowhere)
    if vaddr % 4096 != 0 || size == 0 || size % 4096 != 0 || phys % 4096 != 0 {
        return Err(ObjectError::InvalidRange);
    }
    // 3. Только теперь маппинг
    aspace.map_pages(vaddr, phys, size, rights_to_flags(self.capability.rights),
                     alloc, self.id)
        .map_err(map_vmm_error_to_object_error)?;   // а не «всё → MappingLimitReached»
    self.mappings[slot] = Some(VirtualMapping { vaddr, size });
    self.state = MemoryObjectState::Mapped;
    Ok(slot)
}
```

Три обязательные правки: (а) поиск слота **до** `map_pages`; (б) **не терять**
`VmmError` — сейчас `:277` сворачивает `MappingTableFull` и `NotMapped` в
`MappingLimitReached`, из-за чего диагностика невозможна; (в) добавить
`ObjectError::{NoRights, InvalidRange, StillMapped}`.

Замороженная эталонная модель `kernel-memory-frozen/src/object.rs:132-140` уже
делает (а) правильно — её логика и её тест `mapping_limit_returns_error` (`:282`)
должны стать источником истины и быть перенесены в живой крейт.

**Объём: small** (один файл, ~25 строк) **при условии**, что §3.1 уже сделан
(иначе `map_pages` начнёт писать `FrameOwner` и типы разойдутся). Порядок:
§3.1 → §3.3.

### 3.4 P0-7 — `materialize_with` без пред-проверки ёмкости

**Файлы:** `kernel/src/vmm/address_space.rs:426-477`, `kernel/src/exec/mod.rs:34-81`.

```rust
pub fn materialize_with(&mut self, va: u64, frame: u64, flags: MappingFlags,
                        alloc: &mut dyn PageTableAllocator) -> Result<(), VmmError> {
    let page = va & !0xFFF;
    let Some(m) = self.query(page) else { return Err(VmmError::NotMapped); };
    if m.backing != Backing::LazyAnonymous { return Err(VmmError::InvalidRange); }
    // (1) Ёмкость ПЕРЕД hardware — ровно тот же пре-чекст, что
    //     в resolve_lazy_fault:363-366 (1 кусок → до 3).
    if self.mappings.len() + 2 > MappingSet::capacity() {
        return Err(VmmError::MappingTableFull);   // вызывающий ничего не освободил
    }
    let pieces = { /* head/Present/tail через Mapping::split (§3.2) */ };
    // (2) Точная проверка, а не «+2 сверху»
    if self.mappings.len() - 1 + pieces.len() > MappingSet::capacity() {
        return Err(VmmError::MappingTableFull);
    }
    unsafe { mmu_map_object(self.root, page, frame, 4096, flags, alloc); }
    self.mappings.replace_slots(core::slice::from_ref(&m), &pieces)
        .map_err(|_| VmmError::MappingTableFull)
}
```

**Плюс системная правка в `load_elf`.** На отказ `materialize_with` уже
аллоцированный кадр (`exec/mod.rs:59-72`) просто теряется, а `sys_execve`
(`syscall/process.rs:613-615`) **уже** сделал `unmap_all` до загрузки (это
отдельный дефект аудита P0-5), поэтому откатить execve невозможно даже в
принципе. Нужен guard: `let frame = …; let r = aspace.materialize_with(…); if
r.is_err() { deallocate(frame, 4096); return r; }`.

**Объём: small** (повтор существующего пре-чекста из соседней функции) **+ medium**
для отката кадра в `load_elf`. Рекомендую оба.

### 3.5 P1-a — `free_frame`, reserved и PMM-верификатор

**Файлы:** `kernel/src/pmm.rs`, `kernel/src/verify.rs`.

**Ключевой вывод: P1-a закрывается побочно через §3.1, а не отдельной правкой
`pmm.rs`.** `FrameOwner::Platform` не имеет ссылок, значит `release()` никогда
не вызовет `deallocate` для зарезервированного кадра. Поэтому `free_frame`
(`pmm.rs:65-72`) может остаться как есть — при условии, что ни один новый путь
не обходит `frame_owner`.

Что всё же нужно в `pmm.rs` самостоятельно:

1. `free_page` (`:206-214`) должен отвергать невыровненный адрес: сейчас
   `0x8000_0001` пройдёт как `0x8000_0000`.
2. `is_allocated` (`:226-230`) → добавить `is_free_frame(addr)` и
   `is_reserved(addr)`, чтобы гейты могли утверждать состояние **конкретного**
   кадра, а не агрегат (§5.3).
3. `reserved_count` уменьшать **нельзя** (это была бы вторая правка того же
   места). Правильная форма — считать `free_count()` проходом по битмапу (п. 4).

**`verify_pmm` (`verify.rs:18-31`) переписать** — это единственная защита от
повторения P1-a, и сейчас она тавтологична:

```rust
// вместо вычисления через счётчики — прямой подсчёт свободных битов
pub fn verify_pmm(pmm: &PmmBitmap) -> KernelResult<()> {
    let measured_free = (0..pmm.total_frames()).filter(|&i| !pmm.test(i)).count();
    if measured_free != pmm.free_count() { return Err(/* BitmapCorrupted */); }
    // и free_count() тоже переопределить на measured_free
    Ok(())
}
```

6 строк, и инвариант становится настоящим. Альтернативно — хранить
`Allocated`/`Reserved` двумя битами на кадр (тогда `free_page` может точно
сказать, что он освободил, и отказаться, если кадр reserved); это честнее, но
удваивает битмап (для 512 МиБ это 128 КиБ → 256 КиБ, приемлемо). **Рекомендация:
сначала однобитная схема + `frame_owner`; двухбитную рассматривать только если
`frame_owner` по какой-то причине не сможет запретить `free` на reserved.**

**Объём: small.**

### 3.6 P1-b — root-кадры `sys_fork`

**Файлы:** `kernel/src/syscall/process.rs:80-92, 144-146`,
`kernel/src/vmm/address_space.rs:1105-1140`.

`unregister` (`:1113-1140`) перебирает реестр таблиц, но root в него не входит
**по построению** (ADR-031 §2: «Root frames are never registered — they are
boot-allocated and leak by design», `ADR-031:65-68`). Этот ADR написан для
boot-временных корней. Форковые корни — не boot-временные: они выделяются из
PMM в рантайме (`process.rs:81`) и принадлежат конкретному AS.

Правка учёта: расширить `TableEntry` (`tables.rs:19-34`) полем
`is_root: bool` (или `level == 1`); `sys_fork` после успешного `register_child`
(`process.rs:145`) регистрирует root-кадр; `unregister` при `is_root` делает
`deallocate(frame, 4096)` и заканчивает. Boot-корни (AS 0 и `build_root`-созданные
в гейтах) **не** регистрируются — регистрацию вызывает только `sys_fork`, поэтому
«утечка» boot-корней сохраняется ровно по решению ADR-031 §2.

**Но `unregister` не вызывается ни из одного syscall-пути** (§1, P0-1 аудита).
Поэтому P1-b **нельзя закрыть в одиночку**: либо он закрывается вместе с
teardown процесса (чужая миссия), либо остаётся открытым. Здесь фиксируем
только учёт. **Явно: P1-b зависит от P0-1 и не параллелится.**

**Объём: small** (учёт) + явная зависимость.

### 3.7 P1-c — перекрытия и выравнивание

**Файлы:** `kernel/src/vmm/mapping.rs` (`insert`), `kernel/src/vmm/address_space.rs`.

```rust
// mapping.rs
pub const fn validate(&self, m: Mapping) -> Result<(), MappingError> {
    let r = m.virt_range;
    if r.size == 0 || r.base % 4096 != 0 || r.size % 4096 != 0 { return Err(Unaligned); }
    r.base.checked_add(r.size).ok_or(Overflow)?;
    if matches!(m.backing, Backing::Present | Backing::CoW) && m.pa % 4096 != 0 {
        return Err(Unaligned);
    }
    Ok(())
}

// в insert(), ДО записи в слот:
if self.validate(mapping).is_err() { return None; }
if self.iter().any(|e| e.virt_range.base < r.end() && r.base < e.virt_range.end()) {
    return None;   // I-OWN-5
}
```

**Критически важно:** проверка перекрытия — **только по shadow-кускам**, а не по
железу. Identity-блоки 2 МиБ (`lib.rs:760`, `:824`) намеренно перекрываются с
`map_pages` на идентичных VA (`lib.rs:982`, `:1623`), и shadow о них не знает.
Проверка «нет листа в MMU» сломала бы эти гейты и весь boot. ADR-031 §5 («Virtual
aliases are permitted at the mapping level») это прямо разрешает.

Выравнивание `size` дополнительно уже делается в `unmap_pages`/`protect` через
`VaRegion` (`address_space.rs:148`, `:956`), но `map_pages` (`:106`) идёт мимо.
`MemoryObject::map` (§3.3) проверяет сам — тоже дублирование, но лучше
дублирование, чем дыра; при желании вынести проверку в `AddressSpace::map_pages`
целиком, чтобы был один источник истины.

**Объём: small.**

### 3.8 P1-d — убрать `Vec` с fault/user-путей

**Файлы:** `kernel/src/vmm/address_space.rs`.

`for_hardware_runs` (`:237-268`) — единственный, кто аллоцирует **без
необходимости**: он собирает до 64 кортежей, чтобы их отсортировать. Куски в
`MappingSet` уже в слотах; сортировать максимум 64 элемента фиксированного
размера можно на стеке — и этот проект **уже делает** ровно так в `unmap_pages`
(`:151-167`) и `protect` (`:961-972`):

```rust
const MAX_RUNS: usize = 64;
let mut runs = [(0u64, 0u64, false); MAX_RUNS];
let mut n = 0;
// … заполнить из self.mappings.iter() …
// сортировка на месте: insertion sort, как va.rs:339-354
```

Три `Vec<Mapping>` в `resolve_lazy_fault`/`materialize_with`/`resolve_cow_fault`
(`:389`, `:450`, `:530`) никогда не держат больше **трёх** элементов — заменяются
на `let mut pieces = [Mapping::NONE; 3]; let mut n = 0;` плюс
`replace_slots(&pieces[..n])` (сигнатура уже принимает срез, `mapping.rs:256`).

`duplicate_as` (`:588-604`) держит до 5×64 = 17.9 КиБ. Стек не годится
(5 × 3.5 КиБ поверх 64 КиБ стека ядра). Но три из пяти `Vec` устраняются
**структурно**: `shared_orig` не нужен, если hardware-циклы идут после построения
`parent_new`/`child_pieces` (сейчас он существует только чтобы «держать
lockstep» — комментарий `:601-604`; после §3.1 достаточно итерировать
`parent_new` и брать PA из него самого), и `parent_affected` == `parent_new`
после §3.1. Итого остаются 2 `Vec` ≈ 7 КиБ.

**Альтернатива, которую надо назвать честно:** поднять кучу с 64 КиБ до 256 КиБ
(одна строка `lib.rs:672`) и **не** трогать код. Убрать `Vec` — правильно, но не
бесплатно; увеличить кучу — дёшево и снимает риск OOM, оставляя стоимость (пик
~27 %). **Рекомендация:** сначала `for_hardware_runs` и три 3-элементных split-а
(чистый выигрыш на fault-пути, риск ≈ 0), потом решение по размеру кучи отдельной
правкой.

**Объём: medium.**

### 3.9 P2-b — `register`/`register_child` → `Result`

**Файлы:** `kernel/src/vmm/address_space.rs:1071-1103`,
`kernel/src/syscall/process.rs:145`, 16 мест вызова в `lib.rs`.

```rust
pub fn try_register(root: RootPageTable, flags: AddressSpaceFlags)
    -> Result<AddressSpaceId, VmmError>;              // + Err(AddressSpaceBusy)
pub fn try_register_child(child: AddressSpace, root: RootPageTable)
    -> Result<AddressSpaceId, VmmError>;
```

Старые `register`/`register_child` либо удаляются, либо остаются как
`#[cfg(test)]`-обёртки с `.expect` — иначе паника останется доступной.

`sys_fork` (`process.rs:145-146`) при `Err` обязан откатить всё сделанное:
`free_child_boot_frames(…, Some(root_frame))` (уже написан, `:134-140`), плюс
снять COW-регистрацию (после §3.1 — автоматически, т.к. `replace_slots` будет
вызван только при успешном коммите), плюс снять дескрипторы, уже установленные в
дочерний root. Последнее — реальная работа: `mmu_clone_kernel_half` (`:114`) и
`duplicate_as` (`:130`) уже записали дескрипторы. Прагматично: собрать
`child.as_id`-специфичный `reclaim`, использующий реестр таблиц (они
регистрируются через `table_installed`, `object.rs:102-116`).

**Отдельно про пул.** Сегодня 8 AS не снимаются никогда (§1, P2-b). Два
варианта, и оба нужны:

1. Снятие в `thread_exit`/`waitpid` — это P0-1 аудита, чужая миссия.
2. Пока оно не сделано — **увеличить `MAX_ADDRESS_SPACES`**, потому что
   `sizeof(Option<AddressSpace>) ≈ 9.6 КиБ` × 32 ≈ 300 КиБ BSS. Для QEMU (512 МиБ)
   приемлемо; для X96Q/Sumavision с 16 КиБ boot-стеком и малым RAM — вопрос,
   который должен решить владелец платформенной миссии, а не эта. **Это решение
   здесь не принимается**; фиксируется как требование к владельцу пула.

**Объём: medium** (сигнатуры + 17 мест вызова + откат в `sys_fork`).

### 3.10 Сводка работ

| # | Работа | Закрывает | Объём | Зависит |
|---|---|---|---|---|
| W0 | `HwOps` + выделение крейта `vivanta-vmm` | инфраструктура тестов (§4) | medium | — |
| W1 | `frame_owner` + перевод `PhysOwnership` | P0-2, P0-6a, P0-9(i), P1-a(частично) | large | — |
| W2 | `Mapping::split` + `pa_of` | P0-3 | medium | — |
| W3 | пре-чекст ёмкости в `materialize_with` + откат кадра в `load_elf` | P0-7 | small+medium | — |
| W4 | `MappingSet::validate` (выравнивание + перекрытие) | P1-c | small | W2 (общий `PAGE`) |
| W5 | `verify_pmm` пересчётом проходом по битмапу | P1-a (проверка) | small | — |
| W6 | убрать `Vec` из `for_hardware_runs` и 3-элементных split-ов | P1-d (частично) | medium | W2 |
| W7 | `MemoryObject::map` порядок + `check()` | P0-6b, P2-a | small | W1 |
| W8 | `try_register*` + откат в `sys_fork` | P2-b | medium | — |
| W9 | `is_root` в `TableEntry` | P1-b (учёт) | small | W8 |
| W10 | размер пула AS / освобождение на teardown | P2-b (практика) | small | **чужая миссия (P0-1)** |
| W11 | расширение `verify_hardware_consistency` проверкой PA | P0-3, P0-7 (детект) | small | W0, W2 |

---

## 4. Дизайн тестов

### 4.1 Главная проблема и честный ответ

VMM не имеет хостовых тестов, потому что **shadow-логика не отделена от железа**:
`AddressSpace` напрямую зовёт `vivanta_arch_api::mmu::mmu_map_object`, `mmu_unmap`,
`mmu_protect`, `mmu_leaf_descriptor`, `mmu_clear_table_entry` (`address_space.rs:129,
193, 378, 446, 523, 701, 722, 758, 783, 821, 864, 920`), а это свободные
`extern "Rust"`-функции (`arch-api/src/mmu.rs:103-252`). Формально подменить их
нельзя.

**ADR-015 этого не запрещает.** ADR-015 §3 (`ADR-015:63-77`) запрещает
«абстракцию до второй реализации». Как только появляется **вторая** реализация
шва — а её создаёт ровно хостовый набор тестов — правило выполнено. Проект уже
так делает в трёх местах: `PageTableAllocator` (`arch-api/src/mmu.rs:71`) имеет
**три** реализации (`kernel/src/memory/object.rs:16, 73` + boot),
`FrameAllocator` (`arch-api/src/pmm.rs:14`), `MemoryBackend`
(`kernel/src/memory/resource.rs:106`). Добавление второй реализации — не конфликт
с ADR-015, а буквальное его применение.

**Чего делать не надо:** заводить `trait Arch` / `trait PageTable` / иерархию
«arch-мock». Надо сделать ровно две вещи, в идиоме, уже принятой в этом коде.

**(а) Аллокаторная граница уже есть — её достаточно для большинства тестов.**
`PageTableAllocator` — трейт. `FakePta` в тестах — ~40 строк, **ноль изменений в
продакшн-коде**:

```rust
// kernel/src/vmm/testing.rs — #[cfg(any(test, feature = "vmm-tests"))]
pub struct FakePta { next: u64, live: BTreeMap<u64, u8>, installed: Vec<(u64, u64, usize, u8)> }
impl PageTableAllocator for FakePta {
    fn alloc_page_table_frame(&mut self) -> u64 {
        let f = self.next; self.next += 0x1000;
        *self.live.entry(f).or_insert(0) += 1; f
    }
    fn try_alloc_page_table_frame(&mut self) -> Option<u64> {
        if self.next >= 0x1000_0000 { None } else { Some(self.alloc_page_table_frame()) }
    }
    fn table_installed(&mut self, frame: u64, p: u64, i: usize, l: u8) {
        self.installed.push((frame, p, i, l));
        if let Some(x) = self.live.get_mut(&frame) { *x = l; }
    }
    fn reclaim_page_table_frame(&mut self, frame: u64) { self.live.remove(&frame); }
    // плюс доступors: installed_count(), live_count(), oom_after(n)
}
```

`oom_after(n)` — ключевая опция: она позволяет детерминированно воспроизвести
`MappingTableFull`/`try_alloc` → `None` **на выбранной итерации**, вместо
«аллоцировать 512 МиБ».

**(б) Граница «железных» примитивов — одна функциональная таблица,
устанавливаемая при загрузке.** Проект уже делает ровно это трижды:
`faults::VM_ALLOC_CTX` (`vmm/faults.rs:11-14`), `COW_FREE_BACKEND` (`:20`),
`cow_refcount::FREE_FN` (`cow_refcount.rs:30`). Ни одна из них не трейт. Добавить
четвёртую — согласованно, а не революционно:

```rust
// kernel/src/vmm/hw.rs (новый, ~60 строк)
pub struct HwOps {
    pub map:          unsafe fn(RootPageTable, u64, u64, u64, MappingFlags, &mut dyn PageTableAllocator),
    pub unmap:        unsafe fn(RootPageTable, u64, u64, &mut dyn PageTableAllocator),
    pub protect:      unsafe fn(RootPageTable, u64, u64, MappingFlags, &mut dyn PageTableAllocator),
    pub leaf:         fn(root_pa: u64, va: u64) -> u64,
    pub valid_leaves: fn(table_pa: u64) -> u32,
    pub clear_entry:  unsafe fn(table_pa: u64, index: usize),
}
static mut HW: Option<&'static HwOps> = None;
/// Единственная точка установки: на железе — Aarch64Ops (6 строк-обёрток над
/// arch-api), на хосте — HostOps.
pub fn install(ops: &'static HwOps);
#[inline] pub fn hw() -> &'static HwOps;   // panic с внятным сообщением, если не установлен
```

`AddressSpace` заменяет 12 прямых вызовов на `hw().map(…)` и т. д. Это
**механическая** замена, не рефакторинг логики. Побочный, но главный эффект:
становится тестируемым `verify_hardware_consistency` (`:750-797`), который
сегодня — единственный механический инвариант INV-VM-001 и который сегодня
слеп.

**(в) Модель железа для хоста.** `HostOps` держит
`BTreeMap<(u64 /*root*/, u64 /*va*/), u64 /*descriptor*/>`, а `leaf()` возвращает
битовую маску ровно того вида, который проверяет `verify_hardware_consistency`:
бит 0 = valid, `DESC_AP_MASK|PXN|XN` = права, `ADDR_MASK` = PA. ~60 строк. Это
позволяет писать **свойства** `shadow ⇔ hardware` на хосте.

**(г) Расширение `verify_hardware_consistency` проверкой PA — обязательная часть
дизайна, а не отдельная задача.** Сегодня он читает только права
(`address_space.rs:766-774`). Добавить:

```rust
let expected_pa = m.pa_of(m.virt_range.base + off).expect("hardware-backed piece");
if desc & 0x0000_FFFF_FFFF_F000 != expected_pa {
    return Err(VmmError::InvalidRange);   // pa drift: P0-3 пойман механически
}
```

`mmu_leaf_descriptor` (`arch-aarch64/src/mmu.rs:498-501`) уже возвращает полный
дескриптор; новой arch-API поверхности не нужно. Это 4 строки, которые ловят
P0-3 и «призрачный PA» вообще на любом пути.

**(д) Куда положить, чтобы тесты реально запускались.** `kernel` сейчас не
собирается на хосте (`panic = "abort"` в `vivanta-boot/Cargo.toml:34-35` плюс
`core::arch::asm!` по всему `scheduler`/`lib.rs`). Прагматичный вариант, не
трогающий сборку ядра: вынести `mapping.rs` + `frame_owner.rs` + `hw.rs` +
чистую часть `address_space.rs` в **новый крейт `vivanta-vmm`** (`#![no_std]`,
без `alloc`, зависит только от `arch-api` и `vivanta-vm`), а
`kernel/src/vmm/*` оставить ре-экспортами (`vmm/mod.rs:1-21` уже является
фасадом). Тогда `cargo test -p vivanta-vmm` работает нативно, и CI просто
добавляет крейт в существующую строку `cargo test`. Цена: одно перемещение файлов
+ `pub use`.

**Компромисс, названный честно.** `duplicate_as`, `resolve_*_fault`,
`unmap_pages`, `protect` — методы `&mut self` на `AddressSpace` (≈9.6 КиБ на стек,
`address_space.rs:12`) и они обращаются к `faults::anonymous_backend()`
(`:208`, `:870`) и к `interrupts_guard()` (`:908`). Хостовые тесты смогут покрыть
**методы** `AddressSpace` (внешних глобалов там ровно три, все подменяемы:
`anonymous_backend` — через `set_backing_context` с фейковым `MemoryBackend`,
`interrupts_guard` — no-op на хосте, `tables::record` — своя копия реестра или
реальная, она идемпотентна), но **не** глобальный реестр AS
(`register`/`unregister`/`find_by_root`, `address_space.rs:1057-1218`) и **не**
`reclaim_empty_tables` (ему нужен реальный MMU для `mmu_table_valid_leaves`).
Тесты на реестр и на реклэйм — это QEMU-гейты, §5. Это разграничение надо
записать в ADR, иначе следующий агент будет искать хостовый тест для `unregister`
и потеряет деньги.

### 4.2 Существующие тесты: что затронуто и что не запускается

| Файл | Тестов | В CI? | Затронут |
|---|---:|---|---|
| `vivanta-boot/vm/src/va_tests.rs` | **12** (не 22 — §0) | да | **нет**. `VaAllocator` не меняется. P0-4 уже закрыт здесь. |
| `vivanta-boot/exec/src/elf_tests.rs` | 10 | да | **нет напрямую**: §3.4 меняет `load_elf` (кейтч-гард), но `plan_load`/`LoadPlan` не трогаются. |
| `vivanta-boot/boot-info/src/mmap.rs` | 3 | **НЕТ** | нет. |
| `vivanta-boot/boot_common/src/memory_discovery.rs` | 3 | **НЕТ** | нет. |
| `vivanta-boot/kernel-memory-frozen/src/object.rs` | 7 | **НЕТ** (крейт не член workspace, `vivanta-boot/Cargo.toml:4-32`) | **да, напрямую**: `mapping_limit_returns_error` (`:282`) — прямая регрессия P0-6b. |
| `vivanta-boot/kernel/src/**` | 0 | — | здесь появляются новые. |

**Факт, проверено запуском:** `cargo test --workspace --target <host>` → **28**
зелёных; `cargo test -p vivanta-vm -p vivanta-exec` → **22**.
`.github/workflows/ci.yml:49-51` запускает только второе. То есть 6 тестов
(`boot-info` 3 + `boot_common` 3) компилируются (`cargo check --workspace`,
`ci.yml:57`) и **никогда не исполняются**. Плюс 7 замороженных.

**Минимальная правка CI (отдельная работа, ~1 строка + решение):**
заменить `cargo test -p vivanta-vm -p vivanta-exec` на
`cargo test --workspace`. На x86_64-раннере это сразу покроет 6 пропущенных.
`kernel-memory-frozen` — либо добавить в `members`, либо (правильнее, раз он
«frozen»-эталон) отдельная джоба, которая запускает его тесты и **сравнивает**
результаты с живой моделью; сейчас он тихо разошёлся с ней именно в
`MemoryObject::map` (P0-6b), и это лучший аргумент за такой режим.

**Отдельно:** `.cargo/config.toml` (`vivanta-boot/.cargo/config.toml:1-2`) ставит
`[build] target = "aarch64-unknown-none"`, поэтому хостовые тесты требуют явного
`--target`. На x86_64 CI это работает; локально на macOS/arm нужен
`--target aarch64-apple-darwin` (проверено: без флага — `can't find crate for
core`). Не дефект, но неочевидно — стоит упомянуть в README/CONTRIBUTING,
иначе каждый новый агент это rediscoverит.

### 4.3 Конкретные тесты, которые поймали бы каждый дефект

Все — на хосте, в крейте `vivanta-vmm`, на `HostOps` + `FakePta` + фейковом
`MemoryBackend`, который **записывает** все `deallocate(addr, size)` в вектор.
Это ключевая деталь: `P0-2`, `P0-3`, `P0-6`, `P0-9` — все про «кто и когда
освободил кадр», а не про «есть ли маппинг».

**P0-2 (`External` → COW):**
- `T-P0-2-a external_present_is_not_cow`: `map_pages(va, P, 4096, RW)` →
  `duplicate_as` → **parent и child** оба `Backing::Present`,
  `phys == FrameOwner::Object(id)`, `refs_of(P) == 1`,
  `hw().leaf(child, va) & 1 == 1` и `desc & RW == 0` (RO в HW).
  **Сегодня провалится**: `phys` будет `CoWShared{2}`, refs = 2.
- `T-P0-2-b cow_break_never_frees_external`: `T-P0-2-a` + `munmap(parent)` +
  `munmap(child)` → `fake_backend.deallocated` **пуст**.
  Проверяет конец цепочки (`faults.rs:22-28`) целиком.
- `T-P0-2-c external_alias_fork_teardown`: два `map_pages` на один `P`
  (`MemoryObject` + `map_new_range`), форк, teardown обоих → P **не** освобождён.
  Это сценарий §2.1 шаг 4.
- `T-P0-2-d platform_frame_never_freed`: `FrameOwner::Platform` на кадре,
  принудительный `release()` → `false`, `deallocated` пуст, `sweep() == 1`.
  Связывает P0-2 и P1-a.

**P0-3 (смещение PA):**
- `T-P0-3-a split_rebases_pa`: `map_pages(va, P, 3*4096, RW)`;
  `protect(va+4096, 4096, RO)`; для `off ∈ {0,4096,8192}`:
  `m.pa_of(va+off) == P + off`. Проверяет и head, и covered, и tail.
- `T-P0-3-b fork_preserves_per_page_pa`: то же + `duplicate_as`; для каждой
  страницы `hw().leaf(child_root, va+off) & 0xF_FFFF_FFFF_F000 == P + off`.
  **Главный тест дефекта** — он ловит ровно то, что построил boot-гейт
  `lib.rs:1509-1540` и не проверил.
- `T-P0-3-c cow_break_copies_the_right_page`: 3-страничный кусок, COW-break на
  третьей странице → содержимое `new_frame` == содержимое `P+8192`.
  Ловит `address_space.rs:522`.
- `T-P0-3-d unmap_split_frees_the_right_range`: после расщепления `unmap` хвоста
  → `fake_backend.deallocated == [(P+8192, 4096)]`, **не** `[(P, 4096)]`.
- `T-P0-3-e protect_covered_piece_rebases`: та же проверка для `protect` с
  двусторонним расщеплением (head + covered + tail за один вызов).

**P0-6 (`MemoryObject`):**
- `T-P0-6-a map_checks_slots_before_installing`: 4× `map` OK; 5-я → `Err`; **и**
  `aspace.mappings.len() == 4` **и** `hw().leaf(as, v5) == 0`.
  Прямой порт `kernel-memory-frozen/src/object.rs:282` на живую модель.
- `T-P0-6-b revoke_with_live_mappings_is_refused`: `map`, затем `revoke()` →
  `Err(StillMapped)`, `deallocated` пуст.
- `T-P0-6-c drop_with_live_mappings_leaks_not_corrupts`: `map`, `drop(obj)` →
  `deallocate` **не** вызван; `refs_for_object(id) == 1` (кадр ещё принадлежит
  куску). Документирует выбранную асимметрию «утечка вместо UAF».
- `T-P0-6-d clone_refcounts`: `clone()`, `drop(original)` → кадр **не**
  освобождён; `drop(clone)` → освобождён **ровно один раз**.
  (`object.rs:326-340` сегодня ставит `owns_storage: false` и не имеет
  refcount — тест провалится, и это правильно.)
- `T-P0-6-e drop_after_unmap_frees_once`: `map`, `unmap`, `drop` →
  `deallocated == [(P, 4096)]` ровно один раз. Закрывает «двойное освобождение
  при корректном сценарии».

**P0-7 (ghost leaf):**
- `T-P0-7-a materialize_refuses_when_full`: заполнить `MappingSet` до 63
  `reserve_lazy`, `materialize_with` на 64-й → `Err(MappingTableFull)`,
  `aspace.mappings.len()` без изменений, **`hw().leaf(as, page) == 0`**,
  `deallocated` содержит переданный кадр. Зеркалит существующий `[OOM]`-гейт
  (`lib.rs:309-326`), но для **второй** функции.
- `T-P0-7-b no_ghost_after_failure`: после `T-P0-7-a` → `verify_domain_reverse()`
  проходит. Прямое следствие §2.4 шага 5.
- `T-P0-7-c load_elf_reclaims_on_partial_failure`: `FakePta` с
  `oom_after(k)`; `load_elf` падает на k-й странице → суммарный
  `deallocated == аллоцировано` (баланс), `refs_of` для всех кадров == 0.
  Ловит отсутствие guard-а в `exec/mod.rs:59-72`.

**P0-9 (COW-учёт) — самый важный набор:**
- `T-P0-9-a refs_equal_piece_count_after_split`: `Present{pa}` → `duplicate_as`
  → refs(pa) = 2 → COW-break на 1-й странице 3-страничного куска →
  **refs(pa) == 3** (I-OWN-1), кусков на pa == 3.
  **Прямое утверждение инварианта, самое важное в наборе.**
- `T-P0-9-b last_unmap_frees_once`: 3 куска на pa; `munmap` первых двух →
  `deallocated` **пуст**; `munmap` третьего → `[(pa,4096)]` **ровно один**.
  Сегодня: после **первого** `munmap` реестр уже 0 → `deallocate` сразу.
- `T-P0-9-c no_double_free_on_asymmetric_teardown`: `duplicate_as`,
  `unmap_all(child)`, потом `unregister(parent)` → `deallocated` ровно один.
- `T-P0-9-d three_generations`: 3 поколения форка → refs = 4; teardown в
  обратном порядке → `deallocate` ровно один, и **ни на каком шаге не раньше**.
- `T-P0-9-e registry_exhaustion_degrades_to_leak`: 300 COW-кадров → `inc`
  возвращает `None` → refs неизвестен → `deallocate` **никогда** (не «иногда»).
  Проверяет ветки `cow_refcount.rs:68-70, 79`.
- `T-P0-9-f nested_cow_split_accumulates`: 3 поколения × 2 break-а каждый →
  после каждого `refs == число кусков` (I-OWN-1), в конце `deallocate` ровно
  один. Это property-тест на накопление расхождения, которого сегодня нет.

**P1-a (PMM) — с фейковым битмапом, без MMU:**
- `T-P1-a-a free_reserved_is_rejected`: `reserve(bitmap)`, `free_page(bitmap)` →
  бит **остался**, `reserved_count` не изменился, `free_count()` не изменился.
- `T-P1-a-b free_twice_is_noop`: `alloc`, `free`, `free` → `allocated_count == 0`,
  бит чист, `free_count() == baseline`.
- `T-P1-a-c free_list_invariant_is_not_vacuous`: сначала **проверить, что
  переписанный `verify_pmm` падает на искусственно сбитом счётчике**, и только
  потом проверять, что он проходит на корректном. Тест на сам тест: без него
  следующий агент снова напишет тавтологию.
- `T-P1-a-d unaligned_free_rejected`: `free_page(0x…001)` → `Err`.
- `T-P1-a-e out_of_region_rejected`: `free_page(region_end)` → `Err(InvalidAddress)`.
- Property: 20 000 случайных `alloc`/`free`/`reserve` с проверкой
  `allocated + reserved + free_measured == total` **каждые 512 шагов** — прямой
  приём из `va_tests.rs:260-263`, копируется без изменений.

**P1-c (перекрытия/выравнивание) — негативных тестов не хватает больше всего:**
- `T-P1-c-a insert_rejects_overlap`: `insert(Present[0x1000,+4K))`;
  `insert(Present[0x2000,+4K))` → OK; `insert(Present[0x1800,…))` → `None`;
  `insert(Present[0x0800,+8K))` → `None` (полное перекрытие);
  `insert(Present[0x1000,+8K))` → `None`.
- `T-P1-c-b identity_block_map_is_not_an_overlap`: `map_pages(va=phys, 4096)` на
  разные `pa` — **разрешено** (boot-гейты `lib.rs:982`, `:1623`). Гарантирует,
  что проверка shadow-only и не ломает boot. Этот тест обязателен: без него
  естественная реализация «проверить и железо» убьёт загрузку.
- `T-P1-c-c unaligned_rejected`: `map_pages(va+1, …)`, `map_pages(…, pa+1, …)`,
  `map_pages(…, size=4097)`, `map_pages(…, size=0)` → все `Err`. **Ошибка должна
  быть `InvalidRange`, а не `MappingTableFull`** — это заодно ловит потерю
  `VmmError` в `MemoryObject::map` (`object.rs:277`).
- `T-P1-c-d overflow_rejected`: `map_pages(u64::MAX-0xFFF, …, 0x2000)` → `Err`
  (`VirtRange::end` без `checked_add`, `mapping.rs:16-18`).
- `T-P1-c-e unmapped_range_still_rejected`: после `map_pages(va, pa, 8K)`
  `protect(va+4K, 8K, RO)` → `Err(NotMapped)` (хвост не покрыт) и
  `mappings.len()` без изменений.

**P1-d (Vec) — структурный, не поведенческий:**
- `T-P1-d-a no_global_alloc_on_fault_paths`: подменить `#[global_allocator]` на
  счётчик; прогнать `resolve_lazy_fault`, `resolve_cow_fault`,
  `unmap_pages` (→ `for_hardware_runs`), `protect`, `unmap_all` → счётчик **0**.
  Проверяемо компилятором и счётчиком, а не глазами по коду.
- `T-P1-d-b duplicate_as_peak_bound`: измерить `high_water` счётчика на AS с
  64 кусками → `< 16 KiB` (сейчас ~17.9 КиБ без учёта удвоений). Регрессионная
  граница с конкретным числом.

**P2-a/b:**
- `T-P2-a-a check_is_subset`: все 32 комбинации `MemRights` → таблица
  истинности против эталонного `subset` (property test на 32 точках).
- `T-P2-a-b map_requires_map_right`: `cap.rights.map == false` + `map()` →
  `Err(NoRights)`. **Сегодня `map()` вообще не смотрит на capability**
  (`object.rs:266-271` проверяет только `is_revoked`/`Created`).
- `T-P2-b-a try_register_returns_err_when_full`: зарегистрировать 16 AS → 17-я →
  `Err(AddressSpaceBusy)`, **ядро не упало**. Прямая негативная проверка на
  `address_space.rs:1087, 1102`.
- `T-P2-b-b register_child_rollback`: `try_register_child` → `Err` →
  `cow_refcount::total()` и `frame_owner` **не изменились** (после §3.1
  транзакционность структурна; тест фиксирует это).

**Model-based (главное, чего не хватает) — `T-MODEL-shadow-hw`:**

Держать `BTreeMap<va, (pa, perms, backing)>` — независимую модель «что должно
быть в MMU» — и после **каждой** операции (`map_pages` / `protect` /
`unmap_pages` / `resolve_lazy_fault` / `resolve_cow_fault` / `materialize_with` /
`duplicate_as` / `unmap_all`) сравнивать с (а) shadow через
`MappingSet::iter()`, (б) `HostOps` через `hw().leaf()`.

Это ловит **любой** из P0-3 / P0-7 / P1-c разом, потому что утверждает ровно тот
инвариант, который все три нарушают: `shadow ⇔ hardware ⇔ model`. Модель `pa`
вычисляется как `pa_of(va)`, то есть она независима по построению: если
реализация `split` забудет rebase, расхождение появится немедленно и на первом
же расщеплении, без всякого форка.

Драйвер — детерминированный LCG (приём уже принят в проекте: `va_tests.rs:224-230`
и `:176-182`), 20 000 операций, `model.check()` каждые 512, финальный полный
drain. `CoW` в модели — множество «кусков-владельцев» на каждый `pa`, чтобы
`T-P0-9` проверялся не точечно, а статистически.

**Приватный хост-тест для `pmm.rs` — отдельная деталь.** `PmmBitmap` работает с
`u64`-адресами через сырые указатели (`pmm.rs:8`, `:163`, `:233`) и не имеет
`unsafe`-обвязки, поэтому `#[cfg(test)]`-тест внутри `kernel` на хосте не
запустится. Нужен **небольшой** чистый перенос учёта: вынести
подсчёт/установку бита в чистые функции над `&mut [u8] + base`, а
`PmmBitmap` оставить тонкой обёрткой. Это ~40 строк и делает проверяемым весь
P1-a. Без этого переноса P1-a непроверяем в принципе.

---

## 5. QEMU-гейты

### 5.1 Что в существующих гейтах вакуумно, а что — нет

Честный разбор, потому что часть «слабых мест» в аудите относится к вещам,
которые на самом деле сделаны правильно.

**Настоящие источники вакуумности:**

1. **`check()` (`tools/qemu-gates.sh:99-106`) — это `grep -qF` по подстроке.**
   Любой маркер, напечатанный из ветки, которая не выполнялась, даёт PASS. Ровно
   то, о чём аудит говорит для `[FORK-EL0] child teardown`; там уже починено
   (`lib.rs:476-478`: `let-else` + `panic!`), но **структурная** дыра в `check()`
   остаётся для всех остальных маркеров.
2. **`grep -q` не может отличить «напечатали 5 раз» от «один раз».** Для
   счётчиков это неприемлемо.
3. **Слабые сравнения в самих гейтах ядра:**
   - `lib.rs:1613` `assert!(reclaimed_now >= 1, …)` — принимает полную утечку
     (аудит прав). Спасает соседняя `:1614` `assert_eq!(free_delta, reclaimed_now)`,
     но **маркера у этого гейта нет вообще** — его нет в `MARKERS`
     (`qemu-gates.sh:28-47`), поэтому утечка таблиц не видна CI.
   - `lib.rs:971` `assert!(n <= affected.len())` **после** записи (не баг, §0, но
     та же болезнь «проверка в неправильном месте»).
   - `verify_pmm` (`verify.rs:25-28`) тавтологичен — печатает успех, не проверяя
     ничего.
   - Ни один маркер не печатает `vmm::count()` в **конце** загрузки, хотя
     `MAX_ADDRESS_SPACES = 16` и пул почти полон (§1, P2-b). Следующий гейт,
     добавивший один AS, уронит ядро — и это будет выглядеть как таймаут.

**Что сделано правильно и не требует правки (сказано прямо, чтобы не переделывать):**

- Терминальный маркер (`:48`, проверка `:84-87`, вердикт `:124`) ловит таймаут и
  панику корректно: `finished = 0` ⇒ FAIL. `WARN` на `:115` — информационный,
  он не влияет на код выхода. **Механизм рабочий.**
- `grep -qE 'PANIC|EXCEPTION|G4 FAIL'` (`:80`, `:111`) — ловит панику ядра.
- `[OOM]`-гейт (`lib.rs:309-326`) — сильный: 63 слота, `assert!(!ok)`,
  `assert_eq!(len, 63)`, `assert!(query.is_some())`, всё **вне** `if let`.
- `[E2E]`-гейт (`lib.rs:546-556`) — сильный: три **точных** равенства. Единственная
  его слабость — сценарий симметричен и не содержит COW-break (§2.5 iii).

### 5.2 Структурный запрет на вакуумный PASS

Три механизма, которые вместе делают вакуумный PASS **структурно невозможным**,
а не «будьте внимательны»:

**(A) Непогружаемый маркер-счётчик.** Ядро печатает **каждый** успешный гейт
ровно одной строкой вида `  [GATE:<ID>] #<seq> done` — где `seq` глобальный
монотонный счётчик, инкрементируемый **внутри функции гейта** (указатель на
`static mut GATE_SEQ`, инкремент сразу перед `println!` в последней строке гейта).
Скрипт проверяет: значения `seq` начинаются с 1, **строго возрастают без
пропусков**, последнее равно ожидаемому, и каждый ожидаемый `ID` присутствует
ровно один раз (`grep -c` == 1).

Почему это структурно: нельзя напечатать `#N done`, не выполнив ветку, потому
что `println!` стоит внутри неё, а счётчик растёт там же. Пропущенный гейт ⇒ в
логе нет его ID ⇒ FAIL. Неповторяющийся ID ⇒ FAIL.

**(B) Обязательный хвост с телеметрией.** Терминальная метка заменяется на
`  [GATE:END] #<последний> done total=<N> as_count=<M> tables=<T> cow=<C>`,
и скрипт требует `total == N` **и** конкретные значения `as_count`/`tables`/`cow`
(последние берутся из ожидаемой константы в скрипте, а не «любое число»).
Паника/таймаут ⇒ метки нет ⇒ FAIL. `as_count` — это и есть недостающая телеметрия
пула адресных пространств, без которой P2-b невидим.

**(C) Числовые значения вместо строк.** Каждый новый маркер печатает `key=value`,
и скрипт проверяет **точное равенство** конкретных ключей:

```bash
# qemu-gates.sh — строгие предикаты
kv() {           # marker key expected
  local n
  n=$(grep -cF "$1 $2 " "$LOG" || true)
  if [ "$n" -ne 1 ]; then echo "FAIL: $1 $2 (вхождений: $n, ожидалось ровно 1)"; FAIL=1; fi
}
kv '[GATE:E2E]' 'pmm_delta=0'
kv '[GATE:E2E]' 'tables_delta=0'
kv '[GATE:E2E]' 'cow_delta=0'
# монотонность счётчика
seqs=$(grep -oE '^\s*\[GATE:[A-Z0-9]+\] #[0-9]+' "$LOG" | grep -oE '[0-9]+$' | tr '\n' ' ')
[ "$seqs" = "1 2 3 4 … N " ] || { echo "FAIL: sequence gap"; FAIL=1; }
```

Сравнение `after == before` вместо `after >= before` — единственная форма, которая
не может быть пройдена «слишком хорошим» результатом.

**(D) Обязательный запрет на `if let` вокруг PASS.** Правило для всех новых
гейтов: `println!("… PASS")` обязан быть на **верхнем уровне** функции гейта,
после всех `assert!`/`let … else { panic!() }`. Если проверка требует
опционального значения, отсутствие значения должно быть `panic!`, а не `None`
из `if let`. Существующий пример правильной формы — `lib.rs:476-478`.

### 5.3 Новые маркеры (по дефектам)

| Маркер | Что печатает | Как ловит | Vacuity-proof |
|---|---|---|---|
| `[GATE:PAOFF] pa0=<hex> pa1=<hex> pa2=<hex> ok=1` | **P0-3**: после 3-страничного `map_new_range` + `protect` средней страницы — `m.pa_of(va+off)` для трёх страниц **плюс** сверка `hw().leaf(as, va+off) & ADDR_MASK == pa_off` | P0-3 полностью, включая «смещение потеряно» | (A)+(C): три `pa*` обязаны быть **попарно различны** и возрастать; скрипт это проверяет явно |
| `[GATE:FORKPA] child_pa1=<hex> expect=<hex> ok=1` | **P0-3**: после `duplicate_as` проверить `hw().leaf(child_root, va0+4096) & ADDR_MASK` | то, что не сделал `lib.rs:1509-1540` | (C): сравнение с вычисленным `expect` |
| `[GATE:COWSPLIT] n=3 refs=3 freed=0` | **P0-9**: после COW-split число кусков на `pa` == `refs` == 3, `freed == 0` | P0-9(i) | (C): `refs != n` ⇒ FAIL; `freed=0` ловит досрочное освобождение |
| `[GATE:COWLAST] unmaps=3 frees=1` | **P0-9**: три `munmap` кусков на один кадр ⇒ ровно **одна** `deallocate` | P0-9(i) в чистом виде | (C): `frees=1`, **не** `>=1` |
| `[GATE:EXTCOW] extern_refs=1 freed=0` | **P0-2**: `External`-кусок через `duplicate_as` ⇒ refs **1**, не 2; teardown ⇒ 0 frees | P0-2 | (C) |
| `[GATE:GHOST] leaves_ok=1 maps=0 reverse=1` | **P0-7**: `materialize_with` при полной таблице ⇒ `Err`, `hw().leaf == 0`, `verify_domain_reverse()` OK | P0-7 | (C) + `reverse=1` требует **реального вызова** `verify_domain_reverse` (сегодня он не вызывается на этом пути) |
| `[GATE:OBJMAP] slot=4 err=-N leaf=0` | **P0-6b**: 5-й `MemoryObject::map` ⇒ ошибка, но `hw().leaf == 0` | P0-6b | (C) |
| `[GATE:ALIGN] rejected=3 shadow_delta=0` | **P1-c**: три невалидных `map_pages` (невыровненный va / невыровненный pa / size 4097) отклонены, shadow не изменился | P1-c | (C) |
| `[GATE:PMMRES] reserved_before=N reserved_after=N bitmap_intact=1 free_ok=N` | **P1-a**: `free_page` на зарезервированном кадре не снимает бит; `free_count` совпадает с **прямым проходом** по битмапу | P1-a | (C) + `bitmap_intact` печатает результат **переписанного** `verify_pmm` (§3.5), а не тавтологию |
| `[GATE:ROOT] as_id=N root_freed=1` | **P1-b**: после `unregister` форкового AS его root-кадр возвращён | P1-b | (C) |
| `[GATE:POOL] tried=17 err=-5 alive=16` | **P2-b**: `try_register` при полном пуле вернул `Err`, **ядро не упало**, `vmm::count() == 16` | P2-b | (C) — и этот маркер **физически не может напечататься в текущем коде** (там `panic!`), то есть его отсутствие = FAIL, а не «пропуск ветки» |
| `[GATE:HEAP] fault_path_allocs=0 fork_peak=14336` | **P1-d**: счётчик глобального аллокатора вокруг `resolve_lazy_fault`/`resolve_cow_fault`/`for_hardware_runs`, и пик аллокаций `duplicate_as` | P1-d | (C) — `allocs != 0` ⇒ FAIL. Требует `pub fn` чтения/сброса счётчиков `KernelHeap` (`kernel_heap.rs:30-31` сейчас приватные) |
| `[GATE:END] #N done total=N as_count=M` | терминальный + телеметрия | завершение | (B) |

Порядок появления в `MARKERS` (`qemu-gates.sh:28-47`) должен совпадать с порядком
`kernel_main` — иначе последовательный счётчик (A) не совпадёт, и это само по
себе станет проверкой порядка.

### 5.4 Ужесточение существующих гейтов (точечно)

- `lib.rs:1613` `assert!(reclaimed_now >= 1)` → `assert_eq!(reclaimed_now, 1)`.
  В комментарии `:1598-1601` сказано, что реклеймится ровно один L3, так что
  точное значение обосновано. Добавить маркер `[VM] reclaim l3=<n> delta=<n>`.
- `verify_pmm` (`verify.rs:18-31`) переписать на проход по битмапу (§3.5, §4.3
  `T-P1-a-c`) — иначе гейт «G2: verify_pmm» печатает PASS, не проверив ничего.
- `lib.rs:887` `println!("  Address spaces: {} total", vmm::count())` оставить, но
  **продублировать в `[GATE:END]`** — промежуточное значение (3) ничего не
  говорит о пике.
- `address_space.rs:12-13` дополнить константным assert на
  `size_of::<MappingSet>() == 3592` и `size_of::<Mapping>() == 56`: числа из §1
  сейчас не зафиксированы нигде, и рост структуры обнаружится только по факту.

---

## 6. Порядок и риск

### 6.1 Что должно идти раньше чего

```
W0 HwOps + крейт vivanta-vmm            ← ФУНДАМЕНТ ТЕСТОВ, делать первым
 │      (не трогает логику address_space.rs)
 ├─► W11 проверка PA в verify_hardware_consistency
 │
W1 frame_owner (P0-2 + P0-6a + P0-9i)   ← ФУНДАМЕНТ ВЛАДЕНИЯ; меняет смысл
 │                                           публичного поля Mapping
 ├─► W7 MemoryObject::map + check()      (P0-6b, P2-a)
 ├─► W9 TableEntry.is_root              (P1-b учёт)
 └─► W5 verify_pmm пересчёт             (P1-a: проверка)
        ▲
W2 Mapping::split + pa_of (P0-3)        ← НЕ зависит от W1, но трогает те же
 │                                           строки в address_space.rs
 ├─► W6 убрать Vec                       (P1-d)
 └─► W4 validate (перекрытие+выравнивание)  общий код с W2

W3 materialize pre-check (P0-7)         ← независим, но те же строки
                                            address_space.rs
W8 try_register* (P2-b)                 ← независим по смыслу
 └─► W10 размер пула / teardown          зависит от ЧУЖОЙ миссии (P0-1)
```

### 6.2 Что можно делать параллельно (миссия идёт несколькими агентами)

**Ключевой факт для планирования:** `kernel/src/vmm/address_space.rs` (1218
строк) — единственный файл, который трогают **все** содержательные работы
(W1, W2, W3, W4, W6, W8). Его нельзя отдать двум агентам одновременно. Всё
остальное распределяется чисто.

**Волна 1 — параллельно, разные файлы, ноль конфликтов:**

| Агент | Работа | Файлы |
|---|---|---|
| A | W0: `HwOps` + `testing.rs` + выделение крейта `vivanta-vmm` | `vmm/hw.rs` (новый), `vmm/mapping.rs` (только `pub`/`use`), `vmm/mod.rs`, новый `Cargo.toml` + `Cargo.toml` workspace. **Не трогает логику `address_space.rs`.** |
| B | W5: `verify_pmm` + выравнивание в `pmm.rs` + чистый перенос учёта битмапа | `verify.rs`, `pmm.rs` |
| C | §5.4 + §5.2/§5.3: чинка `lib.rs:1613`, маркеры, `[GATE:*]`-счётчик, `qemu-gates.sh` | `tools/qemu-gates.sh`, `kernel/src/lib.rs` (**только boot-гейты**, не VMM) |
| D | §4.2: `--workspace` в CI + решение по `kernel-memory-frozen` + `MemRights` в README | `.github/workflows/ci.yml`, `vivanta-boot/Cargo.toml` |
| E | W2 + W4 + W6: `Mapping::split`/`pa_of`, `validate`, убрать `Vec` | `vmm/mapping.rs`, `vmm/address_space.rs` — **единственный владелец `address_space.rs` в этой волне** |

**Волна 2 — последовательно, W1 одним владельцем:** `frame_owner` +
перевод `PhysOwnership` + `duplicate_as` по ADR-034 + `MemoryObject::revoke` +
`replace_slots`-единственность. Затрагивает `mapping.rs`,
`address_space.rs`, `object.rs`, `faults.rs`, удаляет `cow_refcount.rs`.
Требует E закончить (иначе конфликт по `mapping.rs` и `address_space.rs`).

**Волна 3 — параллельно:**

| Агент | Работа | Файлы |
|---|---|---|
| F | §4.3: тесты `T-P0-2*`, `T-P0-3*`, `T-P0-6*`, `T-P0-7*`, `T-P0-9*`, `T-P1-c*` на `vivanta-vmm` из волны 1 | только тесты, ядро не трогает |
| G | W7 + W3: `MemoryObject::map` порядок + `check()` + `materialize_with` пре-чекст + guard в `load_elf` | `object.rs`, `capability.rs`, `address_space.rs:426-477`, `exec/mod.rs` — **конфликт с волной 2 по `address_space.rs`**, строго после |
| H | W8 + W9 + W11: `try_register*` + откат в `sys_fork` + `is_root` + проверка PA в верификаторе | `address_space.rs`, `tables.rs`, `syscall/process.rs`, 16 мест в `lib.rs` — **конфликт и с волной 2, и с агентом C по `lib.rs`** |
| I | §4.3: `T-P0-9*`-свойства + `T-MODEL-shadow-hw` + приватный хост-тест `pmm` | тесты + `pmm.rs` (**конфликт с агентом B**) |

### 6.3 Риск по пунктам

| Пункт | Риск | Обоснование |
|---|---|---|
| W1 `frame_owner` | **высокий** | Меняет смысл публичного поля в `Mapping`, затрагивает 5 файлов, удаляет модуль. Риск не в сложности, а в **незаметности**: если где-то забыть `acquire`, получится утечка (тихо); если забыть `release` — тот же P0-9 в новой форме. Страховка — `T-P0-9-*` и `I-OWN-1`, плюс `sweep()` как детектор. Порядок «сначала тесты (W0), потом модель» обязателен. |
| W2 `split`/`pa_of` | **средний** | Локальная замена пяти копий, но ошибка = тихая порча данных. Страховка: `T-P0-3-*` + расширенный `verify_hardware_consistency` (W11) + гейт `[GATE:PAOFF]`. |
| W3 `materialize` пре-чекст | **низкий** | Дословный перенос пре-чекста из соседней функции. Guard в `load_elf` — отдельная маленькая правка, риск низкий. |
| W4 `validate` | **низкий-средний** | Главный риск — **сломать boot** проверкой перекрытий против железа. Отдельно оговорено: только shadow. Обязателен `T-P1-c-b` (identity-блоки разрешены). |
| W5 `verify_pmm` | **низкий** | Чем проще, тем лучше; но без `T-P1-a-c` («тест на сам тест») тавтология вернётся. |
| W6 убрать `Vec` | **низкий** | Механическая замена на стековые буферы по образцу, уже применённому в том же файле. Ошибка = паника на переполнении буфера, то есть заметна сразу. |
| W7 `MemoryObject::map` + `check` | **низкий** | Прямое приведение к замороженной эталонной модели. Риск — **поменять порядок и сломать кейт**; откат тривиален. |
| W8 `try_register*` | **средний-высокий** | 17 мест вызова + откат в `sys_fork`, где уже записаны дескрипторы в дочерний root. Откат — самая тонкая часть; недо-откат = утечка таблиц (безопаснее, чем UAF, но всё же). |
| W9 `is_root` | **низкий** | Аддитивное поле. **Но не закрывает P1-b** без teardown — риск «считать закрытым». |
| W10 размер пула | **средний** | Увеличить `MAX_ADDRESS_SPACES` дёшево, но это решение владельца платформенной миссии (BSS ~10 КиБ/слот, на X96Q/Sumavision RAM мал). Здесь **не принимается**. |
| §4.1 `HwOps` | **средний** | Механическая замена 12 вызовов, но затрагивает `address_space.rs` — конфликт со всем. Отсюда решение поставить его в волну 1 **одним владельцем**. |
| §5 гейты | **низкий** по коду, **средний** по «сломает существующие 18/18» | Новый `[GATE:*]`-счётчик требует, чтобы **каждый** существующий гейт печатал строку в новом формате; если забыть один — терминальный маркер не появится и **всё** станет FAIL. Порядок обязателен: сначала счётчик + все 18 гейтов, потом новые маркеры. |
| §5.4 `assert_eq!(reclaimed_now, 1)` | **средний** | Значение 1 обосновано комментарием `:1598-1601`, но не измерено. Первая попытка может дать 2 и обрушить гейт. **Сначала** заменить на печать `reclaimed_now=<n>`, снять значение, потом ставить точное. |

### 6.4 Что НЕ параллелится ни при каких условиях

- **`address_space.rs`**: W1, W2, W3, W4, W6, W8 — строго последовательно,
  один владелец.
- **`mapping.rs`**: A (волна 1, только `pub`) → E (волна 1, `split`/`validate`) →
  W1 (волна 2, `phys: FrameOwner`) — три разных по времени правки одного типа.
- **`lib.rs`**: агент C (§5 гейты) и агент H (W8, 16 мест вызова) — строго
  последовательно, иначе merge-конфликт в 1210-строчном файле.
- **`pmm.rs`**: агент B (волна 1) и агент I (приватный тест, волна 3) —
  последовательно.

### 6.5 Риск «считать закрытым, а не закрыть» (главный процессный риск)

Два пункта имеют свойство выглядеть закрытыми после узкой правки:

1. **P1-b** после W9 (`is_root`) — учёт есть, но `unregister` из syscall-пути не
   вызывается, то есть фактического освобождения не происходит. **Оставить P1-b
   в бэклоге открытым до решения P0-1 аудита.**
2. **P1-a** после W5 (`verify_pmm`) — детектор есть, но дефект живёт до W1
   (`frame_owner`). **Считать закрытым только после W1.**

И один пункт с обратным свойством — легко пропустить целиком, потому что он
«просто приведение к порядку»:

3. **W11 (проверка PA в `verify_hardware_consistency`)** — 4 строки, но без них
   P0-3 не имеет **механического** детектора ни в одном из двух существующих
   верификаторов. Это самая дешёвая и самая ценная правка во всей миссии по
   соотношению «строк к пойманному дефекту».

---

## 7. Отчёт: что оказалось не так в аудите и одно главное structural-рекомендация

### 7.1 Найдено уже исправленным

- **P0-4 `VaAllocator::reserve()`** — исправлен коммитом `82a2b94` (после даты
  аудита). `vm/src/va.rs:143-149` + `:157-159`; тесты `va_tests.rs:301, 321`.
  В списке задания его не было, отмечено, чтобы не переделывали.

### 7.2 Найдено описанным неточно

- **«22 host-теста в `va_tests.rs`»** (в задании) — в файле **12**. «22» = 12
  (`vivanta-vm`) + 10 (`vivanta-exec`), как и в `docs/audit/2026-09-25-cleanup.md:93`.
- **P0-9** описан как «не-транзакционность». Это верно, но **вторично**. Главная
  поломка — **асимметрия**: COW-split превращает 1 кусок в 2, а реестр падает на
  1, и расхождение накапливается с каждым `resolve_cow_fault`. Именно поэтому
  `[E2E]`-гейт с **точными** значениями зелёный: его сценарий симметричен. Одно
  добавление COW-break перед teardown — и `assert_eq!(pmm.free_count(), free_pre)`
  (`lib.rs:546`) падает. Самая дешёвая регрессия из всех возможных.
- **P0-6a «освобождает кадры при живых маппингах»** — по коду подтверждено, но
  **сегодня недостижимо**: `revoke()` и `MemoryObject::clone()` не имеют ни
  одного вызывающего во всём `kernel/src`, а все аллокаторы делают
  `core::mem::forget(obj)` (`object.rs:30, 83, 98`). Фактическая серьёзность
  сегодня — P2, а не P0; P0 она станет на первом же file-backed mmap.
- **P2-2 «`register` паникует»** оценён в аудите как P2, а на практике это **P1**:
  пул `MAX_ADDRESS_SPACES = 16` уже почти исчерпан (8 AS не снимаются никогда), и
  добавление одного нового гейта, регистрирующего AS, уронит **всю** загрузку —
  причём отказ будет выглядеть как таймаут (`qemu-gates.sh:115` печатает `WARN`).
- **P0-2 «последний владелец освободит кадр, который VMM никогда не владел»** —
  механизм верен, но **ни один syscall-путь сегодня не создаёт
  `Present(External)` в форкабе AS**: `mmap` даёт `LazyAnonymous`, `execve` даёт
  `Anonymous`. Единственное живое место — boot-гейт COW `lib.rs:1317`, где кадр
  случайно взят из `pmm.alloc_frame()` и потому двойное владение *случайно*
  сбалансировано. Фактическая серьёзность сегодня — **P1**, не P0.

### 7.3 Найдено не в аудите вообще (новые находки)

1. **`verify_pmm` («Инвариант 2») — тавтология** (`verify.rs:25-28` против
   `pmm.rs:184`). Гейт зелёный всегда, ничего не проверяет. Это отключает
   единственный механический детектор PMM.
2. **«`DAIFSet #2` маскирует SError, а не IRQ» — неверно.**
   Ранее `arch-aarch64/src/interrupts.rs` противоречил сам себе: строка
   правильно описывала раскладку DAIF, а следующая утверждала
   противоположное. (Комментарий исправлен 2026-09-29.) `imm4`:
   bit3=D, bit2=A (SError), bit1=I (IRQ), bit0=F; значит `#2` = бит I = IRQ,
   `#4` = бит A = SError. P0-железо-пункт 1 и «пункт 1» ночной секции
   аудита («прерывания фактически не маскируются») построены на ошибке чтения.
   Практическое следствие для этой миссии: IRQ **маскируются**, поэтому
   реентерабельного дедлока в спин-локе кучи нет, и P1-d — это OOM/бюджет, а не
   дедлок. **Передать IRQ-миссии.**
3. **`verify_hardware_consistency` не проверяет PA вообще** (`:766-774` сравнивает
   только биты прав). То есть у проекта **нет ни одного** механического детектора
   P0-3, хотя `mmu_leaf_descriptor` уже возвращает нужное поле. Правка — 4 строки.
4. **`MappingSet::len()` завышает число живых кусков** (возвращает high-water
   индекс слота, `mapping.rs:223-225` против `:169-170`). Пред-проверки ёмкости
   на этом эффекте консервативны (безопасно), но любой будущий код, читающий
   `len()` как «сколько кусков», ошибётся.
5. **`protect()` пишет в массив до проверки границы** (`address_space.rs:971`;
   в `unmap_pages` тот же порядок исправлен на `:159-162`). Сегодня безопасно
   только потому, что `MappingSet` ≤ 64 элементов.
6. **6 хостовых тестов (`boot-info` 3 + `boot_common` 3) компилируются и никогда
   не исполняются** (`.github/workflows/ci.yml:49-51` запускает только
   `-p vivanta-vm -p vivanta-exec`). Проверено: `cargo test --workspace` даёт 28
   зелёных. Плюс 7 тестов в `kernel-memory-frozen`, который вообще не член
   workspace — и один из них (`mapping_limit_returns_error`) — прямая регрессия
   P0-6b против живой модели.
7. **`MemoryObject::map()` теряет `VmmError`**: любой отказ `map_pages`
   (`MappingTableFull`, `NotMapped`) сворачивается в
   `ObjectError::MappingLimitReached` (`object.rs:277`), из-за чего
   `MappingTableFull` диагностируется как «слишком много маппингов».
8. **`MemoryObject::map()` игнорирует `capability.rights` полностью**
   (`object.rs:266-271` проверяет только `is_revoked`/`Created`); биты
   `MemRights::map`/`share` не переносятся в `MappingFlags` (`:384-393`) без
   комментария.
9. **Пул адресных пространств телеметрируется только на середине загрузки**
   (`lib.rs:887` — печатает 3), в конце не печатается ничего. Именно поэтому
   P2-b невидим.
10. **[OOM]-гейт не ловит sibling-функцию.** `gate_fault_oom` (`lib.rs:290-331`)
    проверяет `resolve_lazy_fault` на полной таблице; `materialize_with` с
    точно такой же ошибкой не проверяется нигде.

### 7.4 Единственная главная structural-рекомендация

**Заменить пару «счётчик в shadow + глобальный реестр» на один реестр
владельцев с одним владельцем на кадр (`FrameOwner` =
`Platform | Object(id) | Vm { refs }`), и сосредоточить **все** изменения счётчика
в `MappingSet::replace_slots`/`remove` — единственной функции, которая уже
существует и уже является транзакционной точкой.**

Почему именно это, а не «починить `duplicate_as`»:

- Один фикс `duplicate_as` закрывает P0-2, но оставляет P0-9(i) (асимметрия
  COW-split), P0-6a (Drop при живых маппингах) и P1-a (reserved не отличим) —
  потому что все три живут в **разных** местах того же разрыва между «кто
  владелец» и «сколько ссылок».
- Реестр с `refs` **внутри категории владельца** и **запретом на смену категории**
  делает P0-2 непредставимым в типах: `Object(_) → Vm { .. }` невозможно
  написать. Сегодня это запрещено комментарием и ADR, завтра — компилятором.
- Перенос `inc` из `duplicate_as` в `replace_slots` превращает «транзакционность»
  из **документации** (`address_space.rs:674-675`, которая сегодня неверна) в
  **структуру**: состояние реестра меняется ровно в одной функции, рядом с
  проверкой ёмкости. Это единственная правка, которая убирает целый класс
  расхождений, а не один его симптом.
- `Platform`-категория однажды закрывает P1-a, потому что «reserved» перестаёт
  быть свойством битмапа (которое неотличимо от «allocated» одним битом) и
  становится свойством реестра.
- Тот же реестр даёт `refs_for_object(id) == 0` как **проверяемое** условие
  для `MemoryObject::revoke` вместо ручной дисциплины `mem::forget` — то есть
  P0-6a закрывается без изменения `AddressSpace`.

Второе по важности, и в три раза дешевле: **добавить проверку
`m.pa_of(va) == desc & ADDR_MASK` в `verify_hardware_consistency`
(`address_space.rs:750-797`)**. 4 строки, ноль новой arch-API поверхности, и это
единственный механический детектор P0-3 — дефекта, который сегодня не поймал ни
один из 18 прошедших QEMU-гейтов, хотя boot-гейт `lib.rs:1509-1540` буквально
конструирует повреждённое состояние и не проверяет его.
