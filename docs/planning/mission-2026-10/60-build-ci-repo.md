# 60 — Сборка, CI, гигиена репозитория и governance

**Миссия:** mission-2026-10 · **Срез:** HEAD `273f9bc` · **Дата:** 2026-09-29
**Область:** `.github/workflows/{ci,release}.yml`, `vivanta-boot/{build.sh,flash_rk3568.py,tools/,rust-toolchain.toml,Cargo.toml,.cargo/,target-*/.cargo/}`, `docs/{CHANGELOG.md,REPOSITORY_GUIDE.md,audit/,architecture/}`, `vivanta-boot/docs/{adr,rfc,rfcs}/`, `docs/history/`

> **Статус документа:** план, не исполнение. Ни один файл, кроме этого, не изменён;
> ни один GitHub-параметр не тронут. Всё, что помечено **RECOMMENDATION**, — предложение.
> Проверка сделана командами `gh`, `git`, `grep`, `ls` по рабочему дереву и API;
> датам аудита не доверяем.

---

## 0. Что уже исправлено (кратко)

Три пункта аудита 2026-09-24 на срезе `273f9bc` **уже закрыты** и в план работ не входят:

- **git-трекаемые артефакты** (`images/legacy/*.uImage`) — сняты с учёта;
  `git ls-files` не возвращает ни `.bin`, ни `.elf`, ни `.img`, ни `.uImage`.
  (Подтверждение: `git ls-files | grep -iE '\.(bin|elf|img|dtb|dtbo|itb|map|srec|uImage)$'` → пусто.)
- **вендорный `rkdeveloptool`** — удалён из индекса и из `HEAD`
  (`git cat-file -e HEAD:vivanta-boot/tools/rkdeveloptool` → not in HEAD; 205 КБ blob
  остался только в истории — коммит `b4b30a0`).
- **`docs/CHANGELOG.md`** — получил шапку «partial journal» и бэкфилл пропусков;
  исходная претензия «лакуна, нужна шапка ИЛИ восстановление» **MISDESCRIBED**:
  шапка уже стоит (`docs/CHANGELOG.md:3-12`), а разрывы объявлены ожидаемыми.

Остальное ниже — либо CONFIRMED, либо MISDESCRIBED по формулировке, но реально.

---

## 1. Таблица верифицированных пунктов

| ID | статус | доказательство (путь:строка / вывод) | impact | effort | проверяемо сейчас? |
|---|---|---|---|---|---|
| B-1 | **CONFIRMED** | `gh api repos/Egorich-print/vivanta/branches/main/protection` → `{"message":"Branch not protected"}` HTTP 404; `gh api .../rulesets` → `[]` | Нет блокировки merge вообще; QEMU-гейт не может блокировать, даже если станет blocking | M | ДА |
| B-2 | **CONFIRMED** | `.github/workflows/release.yml` не имеет `needs:` на `ci` и не триггерится `workflow_run`; `on:` только `workflow_dispatch` (`:3-15`); checkout по `inputs.tag` (`:28,66`); гейта нет ни в одном джобе | Релиз публикует произвольный тег без зелёного CI на тот же SHA | M | ДА |
| B-3 | **CONFIRMED** | `release.yml:38-39` `cargo build -p …` без `--release`; упаковка из `target/aarch64-unknown-none/debug/` (`:44-45`) | debug-профиль; абсолютные пути checkout попадают в ELF/DWARF | S | ДА |
| B-4 | **CONFIRMED** | `tools/make-rpi3b-image.sh:34` `FW_BASE=…/raspberrypi/firmware/master/boot`; контрольных сумм нет; `:99` `mkfs.vfat -C -F 32 …` **без** `--invariant`. Проверено: локальный `mkfs.vfat` (dosfstools) поддерживает `--invariant` | Плавающий вход + таймстемп в VBR → образ невоспроизводим байт-в-байт | M | ДА (локально есть `mkfs.vfat --invariant`) |
| B-5 | **CONFIRMED** | `rust-toolchain.toml:12` `components = [...,"clippy",...]`; `grep -rn clippy .github/ vivanta-boot/tools/` → **пусто** | clippy-политика задекларирована и не исполняется | S | ДА |
| B-6 | **CONFIRMED** | `vivanta-boot/.cargo/config.toml` задаёт `[build] target`; `target-*/.cargo/config.toml` лежат в подкаталогах, а все скрипты/CI `cd` в `vivanta-boot/` (`build.sh:4`, `qemu-gates.sh:19`, `ci-local.sh:17`). Следствие: `-zmax-page-size=0x1000` (`target-rk3568/.cargo/config.toml:3`, `target-rpi3b-plus/:3`, `target-suma-q5/:7`) **не применяется** — в `build.rs` этих таргетов его нет (`grep -rn max-page-size */build.rs` → пусто) | Мёртвая конфигурация; выравнивание страниц линкера фактически дефолтное | S | ДА |
| B-7 | **CONFIRMED** | `build.sh:50` `echo "  python3 flash_rk3568.py"`; `flash_rk3568.py:13` `BAUD=115200`, `:125,144` `mw.l` + `sleep(0.005)`. Канон: `docs/hardware/rk3568/scripts/flash-serial-mm.py:37` default `--baud 1500000`, `:27` `CHUNK=64`, `:28` `WORD_DELAY=0.06`, `:65` `mm.l`; runbook `uboot-emmc-bringup.md:170` «1,500,000 baud» | `build.sh` указывает на устаревший скрипт; два источника истины | S | ДА |
| B-8 | **ALREADY FIXED** | `git ls-files | grep -iE '\.(bin|elf|img|dtb|itb|uImage|srec)$'` → пусто; `.gitignore:12-21`; commit `b4b30a0` | — | — | ДА |
| B-9 | **ALREADY FIXED** (частично) | `rkdeveloptool` не в `HEAD`/индексе; `rust-objcopy.sh` — **скрипт-обёртка**, не бинарь (`cat` → bash, резолвит из sysroot). НО: 205 КБ blob `vivanta-boot/tools/rkdeveloptool` остался в истории (`git rev-list --objects --all …` → `205344`); `Cargo.lock` и `assets`-релиз без записанных checksum/лицензий прошивки | Остаточный вес в истории; нет provenance-записи для firmware-ассета | S | ДА |
| B-10 | **CONFIRMED** | `grep 'uses:' .github/workflows/*.yml` → все `@v4` (`actions/checkout@v4` ×6, `upload-artifact@v4` ×2, `download-artifact@v4` ×2); `.github/` содержит только `workflows/`; `CODEOWNERS`/`dependabot.yml`/`SECURITY.md`/PR- и issue-шаблонов нет (`ls` → No such file) | Нет supply-chain пиннинга и стандартного governance | M | ДА |
| B-11 | **CONFIRMED** | лог прогона `36140534933` (lint/test/image): `Node.js 20 is deprecated. The following actions target Node.js 20 but are being forced to run on Node.js 24: actions/checkout@v4` | Шум в логах; будущая жёсткая ошибка | S | ДА |
| D-1 | **CONFIRMED** | `vivanta-boot/tests/minimal_test.rs` — 26 строк, `:1` «This is not a real test file… not supported yet»; `grep -rn minimal_test -- …*.rs *.toml *.sh` → только самообъявление. Каталог `tests/` вне workspace | Мёртвый файл, компилируется ничем | XS | ДА |
| D-2 | **CONFIRMED** | `kernel-memory-frozen/` — 788 строк, 7 `#[test]`, **не** член workspace (`Cargo.toml:5-34` его не перечисляет; в `Cargo.lock` отсутствует); но sibling-док `20-memory-correctness.md:1102,1168` называет его **эталоном регрессии P0-6b** | Дублирует живую модель, но содержит 7 тестов, которых нет в CI | M | ДА |
| D-3 | **CONFIRMED** | `user/hello/` + `user/libc/` — 4 файла (`Cargo.toml`, `src`); ни одной ссылки (`grep -rn "user/hello\|user/libc" …` → пусто); заменены `user-init/` | Мёртвый код | XS | ДА |
| D-4 | **CONFIRMED** | `platform-rk3568` — член workspace (`Cargo.toml:20`), в `Cargo.lock`, но `grep -rn platform-rk3568 */Cargo.toml` → единственный потребитель — сам workspace. `target-rk3568/Cargo.toml` зависит только от `boot_common` | Крейт, не линкуемый ни одним таргетом | S | ДА |
| D-5 | **CONFIRMED** | `arch-aarch64/src/lib.rs:13` `pub mod early_mmu; // dead — zero callers anywhere in the tree`; файл 129 строк; `grep -rn early_mmu` → только это объявление | Мёртвый модуль | XS | ДА |
| D-6 | **CONFIRMED** | `boot_common/src/lib.rs`: `MemoryGeometry` (`:164`) — 0 внешних ссылок; `LpaeMode` (`:142`) — только внутри `Architecture::AArch32`; `BootContext` (`:119`) — 0 внешних конструкций (только статик `:125`) | Типы без единой точки конструирования вне модуля | S | ДА |
| D-7 | **CONFIRMED** | `git ls-files docs/history` → 95 файлов; крупнейший blob `5 892 912 B` = `docs/history/mindmaps/MindMap/Создание собственной ОС_files/index-86c718b2.js`; `mindmaps/` логически `7 448 503 B` (7.1 МиБ) из `7 737 741 B` (7.4 МиБ) истории; **всего трекается 9.3 МиБ** | 5.6 МБ минифицированного JS (SPA FlowUs, без контента) в репозитории ядра | S | ДА |
| DOC-1 | **CONFIRMED** | `ADR-033:42-52` таблица syscall `0–6`; код `kernel/src/syscall.rs:24-39` — `0–15`; в ADR нет ни `FORK/WAITPID/.../RT_SIG*` (`grep` → NONE) | ADR противоречит коду (главный recurring-дефект проекта) | S | ДА |
| DOC-2 | **CONFIRMED** | `ADR-035:4` `Status / Proposed`; `:102` `MAX_PROCESSES = 256`; код `scheduler/task.rs:21` `MAX_TASKS: usize = 64`; реальный лимит AS — `vmm/address_space.rs:19` `MAX_ADDRESS_SPACES = 16` | ADR «Proposed» + неверная ёмкость | S | ДА |
| DOC-3 | **CONFIRMED** | `ADR-032:71` «2.1 Resolvable (exactly one class in M6.0)»; во всём файле `grep COW` → 0 вхождений; `ADR-034` добавляет COW write-fault (`ADR-034:63`); `ADR-032:179` `MAX_ADDRESS_SPACES = 8` против кодовых `16` | Два расхождения: класс fault и ёмкость AS | S | ДА |
| DOC-4 | **CONFIRMED** | `specs/rfc/0001-identity-model.md` = «RFC-001: Theseus Identity Model»; `vivanta-boot/docs/rfcs/RFC-001-identity-model.md` = «RFC-001: Persistent Device Identity»; три дома: `specs/rfc/`, `vivanta-boot/docs/rfc/`, `vivanta-boot/docs/rfcs/` | Коллизия номеров + разбросанность | M | ДА |
| DOC-5 | **MISDESCRIBED** (по цифре) | Аудит: «35 of 57 файлов без входящих ссылок». Моя мерка по basename: **20 из 56** в `vivanta-boot/docs/`; 27 из 133 по `docs/` целиком. Методики разные, но «35» не воспроизводится | Реальная проблема индексируемости, но масштаб меньше заявленного | S | ДА |
| DOC-6 | **ALREADY FIXED** | `docs/CHANGELOG.md:3-12` — шапка «partial journal» стоит; пропуск 07-17…09-17 разбит на milestone-секции (`:88-261`) | — | — | ДА |

**Итог:** CONFIRMED — B-1…B-7, B-10, B-11, D-1…D-7, DOC-1…DOC-4; ALREADY FIXED — B-8, B-9(част.), DOC-6; MISDESCRIBED — DOC-5 (и частично B-9, DOC-6 по формулировке).

---

## 2. Целостность CI — предпосылка доверия любой правке

Эта миссия меняет поведение ядра. Без заслуживающего доверия гейта ни одну правку
нельзя валидировать. Порядок работ здесь — **строго раньше** остальных срезов.

### 2.1. QEMU-гейт → блокирующий

Три независимых условия, все обязательны:

1. **Детерминизм гейта.** `continue-on-error: true` (`ci.yml:99`) стоит из-за реального
   рейса: `9` из `21` не-отменённых `main`-прогонов имели `qemu=failure` (≈1 из 2.3,
   `docs/tooling/ci-cd.md:83-84`). Флип флага **без** починки гонки сделает `main`
   красным в большинстве прогонов. Рейс разбирает **sibling-агент**
   (`docs/audit/2026-09-24-global-audit.md:202-256` — окно между
   `CURRENT_THREAD.store(next)` и входом в поток, `:234-236`; heisenbug, нужен
   failpoint, `:237-241`). **Наша задача — форма workflow, не диагностика.**
   Требуемое от sibling-агента: воспроизведение через QEMU `-icount shift=auto,sleep=off`
   + лог прерываний и защита окна. До этого момента флип — преждевременен.
2. **Форма workflow.** Джоб `qemu` уже `needs: [build]` (`ci.yml:89`); после починки
   убрать `continue-on-error: true` (одна строка). Дополнительно рекомендуется
   **сделать гейт устойчивым к одному случайному сбою**: `strategy: fail-fast: false`
   + повторная попытка гейта один раз (`jobs.qemu.steps` обернуть в два прогона либо
   `retries` через `nick-invision/retry`). Но: ретрай **не** заменяет починку —
   при 1/2.3 два независимых прогона всё ещё падают с вероятностью ~0.57² ≈ 33%.
   Поэтому ретрай — только временная подстраховка, не решение.
3. **Branch protection на `main`.** Конфигурация (GitHub API / UI):
   - required status checks: имена джобов **после** починки — `lint`, `test`, `build`,
     `qemu`, `image`;
   - `strict: true` (ветка должна быть в актуальном состоянии с `main`);
   - `enforce_admins: true` (иначе админ-пуш обходит гейт — сейчас в репо `allow_auto_merge=false`,
     `delete_branch_on_merge=false`, правил нет вообще);
   - `required_pull_request_reviews` — минимум 0 сейчас (solo-проект), но включить
     `dismiss_stale_reviews`, когда появятся ревью;
   - запретить force-push и deletion ветки `main`.
   **Проверяемость:** `gh api repos/Egorich-print/vivanta/branches/main/protection`
   после настройки вернёт объект, а не 404. Это acceptance-критерий.

> **Важно (политика репозитория):** branch protection и required-checks — это
> **изменение GitHub-настроек**, а не коммит. В рамках миссии-плана оно **не выполняется**;
> выносится отдельным явным шагом с ручным подтверждением владельца.

### 2.2. Релиз: гейт + требование зелёного CI на тот же SHA

Проблема: `release.yml` собирает по `inputs.tag` (`:28,66`) и **не** зависит от `ci`.
Публикация принимает любой тег `v…`/`M…` (`:115-118`).

Рекомендуемая форма (без боли для ритуалов):

- Триггер: добавить `workflow_run` на завершение `ci` с `types: [completed]`,
  `branches: [main]`; либо оставить `workflow_dispatch`, но **первым шагом**
  джоба `kernel` вызвать GitHub API и проверить, что на SHA тега есть успешный
  прогон `ci.yml`:
  ```
  gh api "repos/$GITHUB_REPOSITORY/actions/workflows/ci.yml/runs?head_sha=$SHA&status=success" \
    --jq '.workflow_runs | length > 0'
  ```
  Если `false` — `exit 1`. Это дёшево (один вызов) и не требует ждать CI.
- Резолвить `SHA` из тега **один раз** (`git rev-parse "refs/tags/$TAG^{commit}"`) и
  использовать его и для checkout, и для проверки, и для сравнения в `publish`.
- Явно требовать, чтобы тег указывал на коммит в `main`:
  `git merge-base --is-ancestor "$SHA" origin/main`.
- В `publish` оставить проверку формата тега (`:115-118`) и добавить, что все
  артефакты построены из того же SHA (метка в имени артефакта = `$SHA`, не только `run_attempt`).

### 2.3. clippy в CI

Кодовая база насыщена `static mut` (46 вхождений) и `unsafe`;
`#![allow(static_mut_refs)]` стоит в `arch-aarch64/src/lib.rs:6` и `kernel/src/lib.rs:2`.

**Рекомендация — консервативный, «только реальные дефекты» уровень:**

1. Начать с `cargo clippy -p vivanta-vm -p vivanta-exec -p vivanta-boot-info -p vivanta-boot-common --target <host> -- -D warnings`
   — только те крейты, что уже тестируются на хосте (не ядро/arch; у них `test = false`
   осознанно, `docs/tooling/ci-cd.md:26-28`). Это даёт сигнал без шквала false-positive
   на `no_std`/`unsafe`.
2. Для ядра/arch — отдельный advisory clippy-прогон (`continue-on-error: true`,
   пока чистка не выполнена), с явным набором allow на уровне workspace-линта:
   `static_mut_refs`, `missing_safety_doc` допускаются; **запрещать** только
   `correctness`-категорию (`clippy::correctness` — по умолчанию deny).
3. Добавить в `rust-toolchain.toml` ничего не нужно — `clippy` уже компонент (`:12`);
   нужен лишь шаг `cargo clippy` в `lint`-джобе (`ci.yml:19-44`) и в `tools/ci-local.sh`.
4. `-D warnings` глобально (для всего workspace) — **не рекомендуется сейчас**:
   при 46 `static mut` и `#![allow(static_mut_refs)]` он гарантированно красный и
   будет отключён. Поэтапно: сначала correctness-only, потом добавлять группы.

### 2.4. Пиннинг actions, Dependabot, Node 20

Проверено по API: `checkout@v4` = `node20`; `upload-artifact@v4` = `node20`;
`download-artifact@v4` = `node20`. `checkout@v5`+ = `node24`;
`upload-artifact@v6`+ = `node24`; `download-artifact@v7`+ = `node24`.

Конкретный миграционный список (минимальный, чтобы убрать Node 20 без смены мажора
поведения):

| Сейчас | Предлагается | SHA (для пиннинга) | Почему |
|---|---|---|---|
| `actions/checkout@v4` | `actions/checkout@d23441…` (v6) или `@3d3c42…` (v7) | v6 `d23441a48e516b6c34aea4fa41551a30e30af803`, v7 `3d3c42e5aac5ba805825da76410c181273ba90b1` | v5+ уже `node24` |
| `actions/upload-artifact@v4` | `@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` (v7) | v6+ `node24` |
| `actions/download-artifact@v4` | `@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c` (v8) | v7+ `node24` |

- Пиннинг: заменить теги на **полные SHA** (комментарий `# vN` рядом), 10 вхождений
  в `ci.yml` (`:22,50,59,74,101,104,135,162`) и `release.yml` (`:26,49,64,90,105`).
- Добавить `.github/dependabot.yml`: `package-ecosystem: github-actions`,
  `directory: /`, `schedule: weekly`, `open-pull-requests-limit: 5`. (Rust-экосистема
  в проекте без внешних крейтов — `Cargo.toml`-зависимостей из crates.io нет; можно
  добавить `cargo` позже, но сейчас он ничего не найдёт.)
- Добавить `.github/CODEOWNERS` (единый владелец `@Egorich-print` на `/.github/`,
  `build.sh`, `tools/`, `docs/adr/`), `.github/SECURITY.md` (приватный канал
  репорта; важно для публичного репо), `.github/PULL_REQUEST_TEMPLATE.md`,
  `.github/ISSUE_TEMPLATE/{bug_report,feature_request}.md`.
- **Как один advisory-джоб:** можно сначала добавить `dependabot` + SHA-пиннинг,
  а смену мажоров делать после проверки, что обновлённые actions работают на
  `ubuntu-24.04-arm` (checkout/artifact кросс-арх — обязательный smoke).

### 2.5. Воспроизводимая сборка — честная планка

Что достижимо и как:

| Аспект | Сейчас | Действие | Достижимость |
|---|---|---|---|
| FAT-образ | `mkfs.vfat` без `--invariant` (`make-rpi3b-image.sh:99`) | добавить `--invariant` | **Полная** — устранение time/random в VBR; `mkfs.vfat --invariant` подтверждён локально |
| Прошивка RPi | плавающий `master` (`:34`), нет checksum | зафиксировать **commit SHA** upstream + записать SHA256 каждого файла в `firmware/SHA256SUMS`; ассет `assets`-релиза уже используется, но без записи сумм | **Полная** для фиксированного набора файлов |
| Пути в ELF | debug-сборка, абсолютный checkout | `--remap-path-prefix=$PWD=/src` (или `CARGO_TARGET_DIR` + `RUSTFLAGS`), сборка `--release` | **Полная** для путей |
| `source_date_epoch` | не задан | `SOURCE_DATE_EPOCH` из `git log -1 --pretty=%ct` тега; экспортировать в сборку | **Частичная** — Rust/LD обычно поддерживают, но не гарантируется для всех секций |
| Временные метки в MBR/Python-обёртке | Python пишет MBR без времени | уже детерминирован (`make-rpi3b-image.sh:109-132` не пишет время) | **Полная** |

**Честная формулировка успеха:** байт-в-байт воспроизводимость **ядра и FAT-образа
на Linux-раннере** достижима; **на macOS** она не гарантируется (разные
`clang`/`ld`-цепочки дают разные секции в `.comment`/notes). Поэтому:

- заявлять: «reproducible **on a fixed Linux toolchain + pinned firmware**»;
- **не** заявлять «identical bytes on macOS and Linux»;
- CI-проверка: собрать дважды с одним `SOURCE_DATE_EPOCH` на Linux и сравнить
  `sha256sum` (`kernel8.img` и `.img`) — это единственное, что доказывает claim.
  macOS-локаль — вне claim.

---

## 3. Мёртвый код и гигиена: план утилизации

**Рамка.** Проект имеет сильный прецедент **архивирования, а не удаления**
(`docs/history/`, `~/ai-workstation/Archives/Vivanta/`) и правило ADR-011
(«Frozen systems — archived to RFC, not deleted»; `STATUS.md:47-48,254`).
ADR-011 и гигиена **конфликтуют** ровно в одном месте: ADR-011 сохраняет
**каталог**, а не **отслеживаемый мусор внутри него**. Разрешение конфликта ниже:
архивируем то, что несёт знание; удаляем то, что знание дублирует или является
vendor-бандлом без контента.

| Объект | Решение | Обоснование | Точное действие |
|---|---|---|---|
| **5.6 МБ minified JS** + весь `_files/` scraper-bundle (7.1 МиБ) | **удалить бандл, оставить PNG + текстовый конспект** | Проверено содержимое: это SPA FlowUs; HTML даёт лишь 1354 символа видимого текста, контент подгружается JS. Реальный контент mindmap — в `Создание собственной ОС.png` (450 926 B). JS-бандл (5 892 912 B, ~60% всех байтов репо) не несёт информации, которой нет в PNG | `git rm -r "docs/history/mindmaps/MindMap/Создание собственной ОС_files"` и `git rm "docs/history/mindmaps/MindMap/Создание собственной ОС.html"`; рядом положить `README.md` со ссылкой на PNG и 1-абзацным (≤1 КБ) текстовым конспектом структуры mindmap. **Прямая рекомендация `docs/architecture/repository-layout.md:230-233` уже разрешает это.** |
| `docs/history/` (остаток ~0.3 МиБ) | **оставить** | ADR-011 + `docs/history/README.md`; без JS это уже не проблема (9.3 МиБ → ~3.4 МиБ трекаемого) | ничего |
| **`kernel-memory-frozen/`** (788 строк, 7 тестов) | **оставить, но подключить к CI отдельной джобой** (не членом workspace) | ADR-011 велит сохранить; sibling `20-memory-correctness.md:1102,1168` доказал: это **живой эталон**, поймавший регрессию P0-6b (`mapping_limit_returns_error`), тесты которого CI не гоняет. Удалять = потерять готовый тест-оракул. Добавлять в `members` нельзя (contract drift, `REPOSITORY_GUIDE.md:139-145`) | отдельный CI-шаг: `cd kernel-memory-frozen && cargo test` (крейт самодостаточен, зависит только от `boot_common`) + **сравнение** его результатов с живой моделью, как предлагает sibling |
| `vivanta-boot/tests/minimal_test.rs` (26 строк) | **удалить** | Само-объявлен «not a real test file» (`:1`), собрать его нельзя, доказательной ценности ноль. Это не «архивный RFC-снапшот», а случайный черновик | `git rm vivanta-boot/tests/minimal_test.rs` (каталог `tests/` станет пустым — удалить) |
| `user/hello/` + `user/libc/` (4 файла) | **архивировать, затем удалить из репо** | Заменены `user-init/` (единственная EL0-программа, embedded через `include_bytes!`). Код несовместим с контрактами ядра (edition 2021 vs 2024) | `git rm -r vivanta-boot/user/`; при желании — копия в `~/ai-workstation/Archives/Vivanta/user-pre-init/` |
| **`platform-rk3568`** (член workspace, ничей) | **удалить из members, архивировать крейт; НЕ удалять физически сразу** | Согласование с `40-silicon-blockers.md:568,576`: RK3568 **паркуется** в пользу RPi 3B+. `target-rk3568` не линкует ядро и зависит только от `boot_common`, так что `platform-rk3568` не нужен **ни одному таргету**. Но он «правильной формы» и пригодится при распарковке RK3568 (NS16550-драйвер). Компромисс: убрать из `members` (`Cargo.toml:20`), перенести в `vivanta-boot/archive/platform-rk3568/`, перезаписать `Cargo.lock` | `git mv` + правка `Cargo.toml`; `cargo check --workspace` как проверка |
| `arch-aarch64/src/early_mmu.rs` (129 строк) | **удалить модуль** | `lib.rs:13` сам помечает «dead», 0 вызовов. НЕ RFC-снапшот — просто осиротевший модуль в живом крейте | `git rm` + удалить строку `pub mod early_mmu;` |
| `boot_common` dead-типы (`MemoryGeometry`, `LpaeMode`, `BootContext`) | **удалить `MemoryGeometry`; `LpaeMode`/`BootContext` — по анализу** | `MemoryGeometry` (`:164`) — 0 ссылок где угодно. `LpaeMode` жив только внутри `Architecture::AArch32` (сам enum — диагностический). `BootContext`+`BOOT_CONTEXT` ещё используется в `target-suma-q5` (паркуется) и в archive — **оставить** до распарковки | `git rm`/убрать только после `cargo check --workspace` + `cargo check -p vivanta-target-suma-q5` |

**Механическая детекция мёртвых крейтов — рекомендация:**

- `cargo-udeps` — **не подходит** для этого workspace: все зависимости path-локальные,
  а таргеты `no_std`/`no_main` (udeps требует ночного компилятора и плохо работает
  с `-Z`-флагами и кастомными targets).
- Вместо этого — **дешёвый CI-скрипт** `tools/dead-code-check.sh`:
  1. для каждого каталога-крейта проверить, что он либо в `members`, либо осознанно
     в архиве (whitelist);
  2. `grep -rn '#\[allow\|// dead'` — собрать само-объявленные dead-модули в отчёт;
  3. для каждого члена workspace проверить, что на него ссылается хоть один
     `target-*/Cargo.toml` **или** он библиотечный (по префиксу `vivanta-` и наличию
     `[lib]`) — иначе предупреждение.
  Это закрывает ровно тот класс, что породил `platform-rk3568`, без внешнего инструмента.
- `cargo machete` (аналог udeps для неиспользуемых зависимостей) — можно, но вторично.

---

## 4. Консолидация скриптов и инструментов

### 4.1. `flash_rk3568.py` vs канонический runbook

**CONFIRMED** расхождение: `flash_rk3568.py:13` `BAUD=115200`, `:125` `mw.l` +
`sleep(0.005)` против канона `flash-serial-mm.py` (`--baud 1500000`, `mm.l`,
`CHUNK=64`, `WORD_DELAY=0.06`) и runbook `uboot-emmc-bringup.md:170`.

**Рекомендация — удалить `flash_rk3568.py`, оставить `flash-serial-mm.py`:**
- `flash_rk3568.py` противоречит runbook **и** технически слабее (`mw.l` без
  FIFO-дисциплины на 115200 коллизионно опасен);
- `flash_rk3568.py` содержит **собственный** генератор DTB (`build_dtb`, `:18-85`) —
  уникальную функциональность, которой нет в каноник-скрипте. Перед удалением
  **вынести генератор DTB в отдельный `tools/mk-dtb-rk3568.py`**, если он нужен
  (проверить у владельца: sibling `40-silicon-blockers.md:44` уже фиксирует SB-10
  как «сверить два скрипта»).
- Правка `build.sh:50`: печатать `python3 docs/hardware/rk3568/scripts/flash-serial-mm.py images/vivanta-rk3568.bin`.
- Строку `build.sh:53` (`mm.l` manual) оставить — она согласована с каноном.

### 4.2. Per-target `.cargo/config.toml`

**CONFIRMED мёртвые** (cargo читает config из invocation-директории, а все скрипты
`cd` в `vivanta-boot/`). При этом `-zmax-page-size=0x1000` **отсутствует в `build.rs`**
→ флаг реально потерян (`target-rk3568`, `target-rpi3b-plus`, `target-suma-q5`).

**Рекомендация — удалить `.cargo/config.toml` в target-каталогах и перенести
flags в `build.rs`:**
- каждый `target-*/build.rs` уже печатает `-T…/linker.ld`;
- добавить туда `println!("cargo:rustc-link-arg=-zmax-page-size=0x1000");` там, где
  он был задуман (rk3568, rpi3b-plus, suma-q5);
- после переноса удалить `target-*/.cargo/config.toml` (5 файлов, подтверждены в HEAD).
- **A/B-доказательство** (что флаг теперь применяется): сравнить
  `readelf -lW kernel8.img`/`llvm-objdump` до и после — изменится `Align` сегментов;
  это и есть требуемое sibling-доказательство (документировать в `ci-cd.md:193`).
- `archive/boot_legacy/**/.cargo/config.toml` — **оставить** (архив, не собирается).

### 4.3. Остаточные вендорные бинарники

- `tools/rust-objcopy.sh` — **не бинарь**, а bash-обёртка, резолвящая `rust-objcopy`
  из `llvm-tools` активного тулчейна. Provenance: компонент тулчейна, лицензия —
  Apache-2.0/MIT (Rust). **Оставить.**
- Единственный оставшийся в **истории** вендорный объект — 205 КБ blob
  `vivanta-boot/tools/rkdeveloptool` (удалён из HEAD). Он **не** достижим ни из одного
  дерева; «убрать из истории» потребует `filter-repo`/force-push — **не рекомендуется**
  (публичная история, ломает клоны, `docs/REPOSITORY_GUIDE.md` ценит историю). Оставить.
- **Записать provenance в репо:** создать `docs/hardware/rk3568/artifacts/PROVENANCE.md`
  со строками: `rkdeveloptool` — удалён коммитом `b4b30a0`, собирается из
  `uboot-emmc-bringup.md:121-125`; `rpi3b-firmware-2026-09-15.tar.gz` (ассет релиза
  `assets`) — Broadcom firmware, лицензия LICENCE.broadcom, SHA256 в
  `firmware/SHA256SUMS` (см. §2.5).

---

## 5. Governance ADR и документации

### 5.1. Конкретные правки ADR

**ADR-033 (syscall ABI), `vivanta-boot/docs/adr/ADR-033-syscall-abi.md`:**
- §3 таблица `:42-52` — заменить `0–6` на `0–15` по `kernel/src/syscall.rs:24-39`:
  `7 FORK`, `8 WAITPID`, `9 KILL`, `10 GETPID`, `11 GETPPID`, `12 EXECVE`,
  `13 RT_SIGACTION`, `14 RT_SIGPROCMASK` (reserved→`-ENOSYS`),
  `15 RT_SIGRETURN` (reserved→`-ENOSYS`). Отметить, какие implemented, какие reserved.
- Добавить аддитивные errno из sibling `30-process-lifecycle.md:284`:
  `-EAGAIN=-11` (fork при полной таблице) — с пометкой «added after freeze».
- §2 «Register contract» (`:36`): уточнить, что `x19–x28` кали-сохраняются (sibling
  `30:53` нашёл, что `execve` их затирает — это **отдельный** дефект, но ADR должен
  явно закрепить контракт).

**ADR-035 (process lifecycle), `vivanta-boot/docs/adr/ADR-035-process-lifecycle.md`:**
- `:4` `Status / Proposed` → `Accepted (amended 2026-09-29)` либо `Partially implemented`:
  fork/waitpid/kill/getpid/execve **живут** с M10.2 (сам файл признаёт это в Note `:165-171`).
- `:102` `MAX_PROCESSES = 256` → зафиксировать, что лимит = `MAX_TASKS = 64`
  (`scheduler/task.rs:21`), а **реальный** предел процессов = `MAX_ADDRESS_SPACES = 16`
  (`vmm/address_space.rs:19`), как вывел sibling `30:839`. Убрать «256 (configurable)».
- Явно записать выбранную асимметрию teardown (sibling `30:150-151`).

**ADR-032 (fault policy), `vivanta-boot/docs/adr/ADR-032-user-vm-fault-policy.md`:**
- `:71` «2.1 Resolvable (exactly one class in M6.0)» → «Resolvable classes (M6.0: one;
  M7.x+: two — demand-fill **and** COW write-fault)» со ссылкой на ADR-034.
- `:179` `MAX_ADDRESS_SPACES = 8` → `16` (`vmm/address_space.rs:19`).
- Добавить ссылку «см. ADR-034 §3 для классификатора COW», чтобы читатель не
  считал §2.1 исчерпывающим.

**ADR-018/ADR-019** тоже `Proposed` (`:3`), но это вне заявленного списка — отметить
как «тот же класс долга», не править в этой миссии.

### 5.2. Три дома RFC и два RFC-001

**Факт:** `specs/rfc/0001-identity-model.md` = «Theseus Identity Model» (draft, pre-R2);
`vivanta-boot/docs/rfcs/RFC-001-identity-model.md` = «Persistent Device Identity»
(design intent). Плюс `vivanta-boot/docs/rfc/` (singular, живой vision-документ).
`specs/rfc/README.md:101-109` уже формулирует правила, но они **не соблюдены**
(`specs/rfc/` по-прежнему имеет три коллизии: RFC-001, `0003` vs `008`, смена ширины).

**Предлагаемый канонический layout:**
- `vivanta-boot/docs/rfcs/` — **единственный** дом **спекулятивных** RFC (RFC-NNN, живой,
  с числом). Сюда пере-нумеровать `RFC-001` из `vivanta-boot/docs/rfcs/` при конфликте.
- `vivanta-boot/docs/rfc/` (singular) — **только** vision/exploration-документы без
  номера (`network-services-vision.md`). Это уже так.
- `specs/rfc/` — **заморозить**: добавить `FROZEN`-баннер в каждый файл и переименовать
  каталог в `specs/rfc-history/` (или оставить, но с явным баннером в README, который уже есть).
- **Коллизия RFC-001:** переименовать исторический `specs/rfc/0001-identity-model.md`
  в `0001-theseus-identity-model.md` (по названию, а не номеру) **или** пометить в его
  шапке «superseded by `vivanta-boot/docs/rfcs/RFC-001-identity-model.md`». Рекомендую
  второе — дешевле и сохраняет историю номеров.
- **Конкретная миграция:** `git mv vivanta-boot/docs/rfcs/RFC-001-identity-model.md
  vivanta-boot/docs/rfcs/RFC-001-persistent-identity.md` + правка ссылок в ADR-011/
  `current-architecture.md`; в `specs/rfc/0001-identity-model.md` добавить строку
  «Superseded by …» без пере-нумерации.

### 5.3. Индексы

- **ADR-индекс:** добавить `vivanta-boot/docs/adr/README.md` с таблицей
  `ADR | Title | Status | Related | Implemented by` (18 строк). Это прямо убивает
  проблему «Proposed-документ, который на самом деле реализован»: статус в таблице
  должен сверяться с кодом.
- **`docs/`-индекс:** добавить `docs/README.md` и `vivanta-boot/docs/README.md` —
  по одному экрану, со ссылками на канонические источники (уже есть в
  `REPOSITORY_GUIDE.md:71-82` — туда и ссылаться). Цель — сократить 20–27 файлов
  без входящих ссылок до единиц.

### 5.4. Правило против рецидива — механический guard

**Корневая проблема:** документы утверждают то, чего код не делает. Предлагаю
**один** дешёвый CI-скрипт `tools/docs-consistency.sh`, который парсит
**машиночитаемые якоря** и падает при расхождении:

1. **Syscall-таблица ADR-033 ↔ код.** В `ADR-033` заменить markdown-таблицу на
   fenced-блок со строкой-якорем и заставить скрипт её проверять:
   ```
   <!-- ADR-033:SYS_TABLE_START -->
   | 0 | READ | ...
   ...
   <!-- ADR-033:SYS_TABLE_END -->
   ```
   Скрипт извлекает строки `| num |`, а из `kernel/src/syscall.rs` — `pub const SYS_(\w+): u64 = (\d+);`,
   и сравнивает множества. Расхождение → `exit 1` с diff.
2. **Ёмкости ADR ↔ код.** То же для `MAX_TASKS` (ADR-035), `MAX_ADDRESS_SPACES`
   (ADR-032), `MAX_MAPPINGS` (ADR-032/`memory/object.rs:128`): регекс по ADR
   (`MAX_TASKS = 64`) против кода.
3. **RFC-номера уникальны.** Скрипт собирает `RFC-\d+` из имён файлов в трёх домах
   и падает при дубле.
4. **Мёртвые пути.** Скрипт проверяет, что каждый путь, упомянутый в
   `REPOSITORY_GUIDE.md` как существующий, существует (`git cat-file -e`).
   Это ровно тот дефект, что был с `archive/`, `theseus-m1/`, `docs/adr/`.

Встраивание: добавить шаг в `lint`-джоб (`ci.yml:19-44`) **после** починки ADR
(иначе скрипт сразу красный), и в `tools/ci-local.sh`. Скрипт — «правило одного
экрана», ~80 строк bash/python.

---

## 6. Срезы работ, параллелизм, порядок, риск

### 6.1. Сериализационные точки

**`S0` (serial):** правки `.github/workflows/` и `vivanta-boot/build.sh` — от них
зависят все остальные срезы. Нельзя параллелить с ними. **Также всё, что трогает
CI-джобы, должно идти после `S0`.**

```
S0 (workflows + build.sh)  ─┬─→ S1 (branch protection, ПОсле починки гонки)
                            ├─→ S2 (release gate + same-SHA)
                            ├─→ S3 (clippy)
                            ├─→ S4 (actions pinning + dependabot + governance)
                            ├─→ S5 (reproducible build)
                            ├─→ S6 (flash-скрипты + .cargo/config)
                            ├─→ S7 (dead code — гигиена)
                            ├─→ S8 (ADR/rfc/индексы + docs-consistency)
                            └─→ S9 (RFC-миграция + индексы)
```

### 6.2. Срезы

| Срез | Содержание | depends | параллельно с | риск |
|---|---|---|---|---|
| **S0** | workflow-форма: ретрай-обёртка гейта, `needs`, подготовка развязки clippy/action-версий; правки `build.sh` (flash). **Не** флипает `continue-on-error` | — | — | **Средний**: ошибка в workflow ломает весь CI. Требует ручной проверки на `workflow_dispatch` до merge |
| **S1** | Branch protection + required checks (GitHub settings; **ручной шаг владельца**) | S0 + починка гонки (sibling) | — | **Низкий** (настройка); **высокий** если включить до починки гонки — `main` красный |
| **S2** | release.yml: same-SHA green + ancestry + артефакт по SHA | S0 | S3,S4,S5,S6,S7,S8,S9 | **Средний**: неверная проверка может заблокировать легитимный релиз |
| **S3** | clippy correctness-only + advisory для ядра; `ci-local.sh` | S0 | S2,S4,S5,S6,S7,S8,S9 | **Низкий** — additive джоб |
| **S4** | SHA-пиннинг actions + dependabot + CODEOWNERS + SECURITY + шаблоны | S0 | S2,S3,S5,… | **Средний**: смена мажоров действий — нужен smoke на `ubuntu-24.04-arm` |
| **S5** | `--invariant`, pinned firmware + SHA256, `--remap-path-prefix`, `source_date_epoch`, двойная сборка-сравнение | S0 | S2,S3,S4,S6,… | **Средний**: `--remap-path-prefix` в debug может ломать отладку — делать в `--release` |
| **S6** | удалить `flash_rk3568.py` (вынести DTB-генератор), `build.sh:50` → канон; перенести `-zmax-page-size` в `build.rs`, удалить `target-*/.cargo/config.toml` | S0 | S2,S3,S4,S5,S7,S8,S9 | **Средний**: неверный флаг линкера ломает образ; проверять `readelf` |
| **S7** | гигиена: `git rm` JS-бандл+HTML (+README), `tests/minimal_test.rs`, `user/`, `early_mmu.rs`, `MemoryGeometry`; `platform-rk3568` → archive; `kernel-memory-frozen` в CI | независим от workflows | S2…S6,S8,S9 | **Низкий по коду, средний по решению** (архив vs удаление) |
| **S8** | ADR-033/035/032 правки + `docs-consistency.sh` + индексы | S0 (для шага CI) | S2…S7,S9 | **Низкий**; риск — «поправить ADR под код» бездумно; сверить с sibling-доками |
| **S9** | RFC-миграция (RFC-001, баннер `specs/rfc/`), ADR-индекс | S8 (ссылки) | S2…S7 | **Низкий** — переименования + ссылки |

### 6.3. Порядок и риски уровня миссии

1. **S1 (branch protection) — только после починки гонки.** Аварийный откат:
   снять required-check `qemu` из protection (30 секунд, GitHub UI) — вернуть
   advisory-статус.
2. **S0 первым.** Ни один другой срез не мержится, пока форма workflow не стабильна.
3. **S2 (release gate) можно делать сразу после S0** — релиз не зависит от
   QEMU-гейта, только от «зелёного CI на SHA»; это безопасно и полезно немедленно.
4. **Риск «план против sibling-доков»:** S6 (RK3568) и S8 (ADR-035/033) пересекаются
   с `20-memory-correctness.md`, `30-process-lifecycle.md`, `40-silicon-blockers.md`.
   **Согласование обязательно:** S6 следует рекомендации `40:576` (парковка RK3568);
   S8 берёт формулировки из `30:808-855`, `20:...`.
5. **Риск «branch protection мешает solo-работе»:** `enforce_admins: true` заблокирует
   прямой push владельца в `main`. Решение: разрешить `bypass` для владельца
   **явно** (GitHub rulesets) или оставить `enforce_admins: false` — но тогда гейт
   не защищает от самого владельца. Рекомендация: `enforce_admins: true` + вся работа
   через PR (это и есть цель миссии).

---

## 7. Ответ на задание

**Что уже исправлено (не трогать):**
- git-трекаемые артефакты `images/legacy/*.uImage` — сняты с учёта (B-8);
- вендорный `rkdeveloptool` — удалён из HEAD (остался только blob в истории, B-9);
- `docs/CHANGELOG.md` — шапка «partial journal» + бэкфилл пропусков (DOC-6).

**Самый высоколевереджный CI-изменение:**
**Branch protection на `main` с required status checks** (B-1). Без него блокирующий
QEMU-гейт не может блокировать merge — а миссия меняет поведение ядра, которое
валидируется только этим гейтом. Но флип гейта в blocking **строго после** починки
scheduler-гонки (sibling), иначе `main` красный при ~1/2.3.

**Рекомендации по утилизации мёртвого кода:**
- **удалить:** 5.6 МБ JS-бандл + scraper `_files/` (оставить PNG + конспект),
  `tests/minimal_test.rs`, `user/hello`+`user/libc`, `early_mmu.rs`, `MemoryGeometry`;
- **архивировать:** `platform-rk3568` (`git mv` в `archive/`, убрать из `members`);
- **оставить, но подключить к CI:** `kernel-memory-frozen` — это живой эталон
  регрессии P0-6b с 7 тестами, не мусор (ADR-011 + sibling `20:1102,1168`);
- **оставить:** остаток `docs/history/` без бандла уже не проблема (9.3 → ~3.4 МиБ).
