# Repository Guide

> Как устроен репозиторий Vivanta и куда что класть.
> Проверено по рабочему дереву 2026-09-29 (HEAD `273f9bc`).
> Если путь отсюда не открывается — это ошибка этого файла, а не дерева.

```
Vivanta/
│
├── README.md / README.ru.md   ← что это и как начать
├── STATUS.md                   ← текущее состояние (коротко)
├── ROADMAP.md                  ← публичная сводка по этапам
├── LICENSE                     ← GPLv3
│
├── docs/                       ← документация уровня проекта
│   ├── REPOSITORY_GUIDE.md     ← этот файл
│   ├── CHANGELOG.md            ← история изменений (НЕ в корне)
│   ├── OS_MATURITY.md
│   ├── manifesto.md
│   ├── architecture/           ← архитектура и мастер-план
│   ├── audit/                  ← результаты аудитов, бэклог дефектов
│   ├── archive/                ← заброшенное исследовательское направление
│   ├── experiments/            ← активные эксперименты (EXP-001, EXP-002)
│   ├── hardware/               ← документация конкретных плат
│   ├── history/                ← ЗАМОРОЖЕННЫЙ архив до-R2 (не редактировать)
│   ├── milestones/             ← завершённые этапы (M1–M4, R2)
│   ├── research/               ← исследовательские заметки
│   └── tooling/                ← документация CI/CD
│
├── specs/                      ← RFC до-R2 и схемы. НЕ авторитетно
│   ├── rfc/                    ← 11 RFC проекта Theseus
│   └── schemas/                ← 2 файла, схемы не содержали никогда
│
├── .github/workflows/          ← ci.yml, release.yml
│
└── vivanta-boot/               ← весь Rust-код и инженерная документация
    ├── Cargo.toml              ← cargo workspace, 20 участников
    ├── kernel/                 ← ядро
    ├── arch-api/               ← ACS: граница ядро↔архитектура
    ├── arch-aarch64/           ← AArch64: векторы, GIC, MMU, таймер
    ├── arch-armv7a/            ← 8-строчная заглушка (ADR-011)
    ├── boot-info/ boot_common/ ← контракт загрузчика и общий boot-код
    ├── vm/ exec/               ← VA-аллокатор и ELF64-загрузчик
    ├── platform-*/             ← платформы
    ├── target-*/               ← бинарные цели
    ├── user-init/              ← ОТДЕЛЬНЫЙ workspace, единственная программа EL0
    ├── tools/                  ← скрипты сборки, гейт, прошивка
    ├── docs/                   ← ВТОРОЕ дерево документации, здесь ADR
    └── archive/, user/, tests/, kernel-memory-frozen/  ← вне workspace, не собираются
```

## Куда что класть

| Что | Куда |
|-----|------|
| Новое архитектурное решение (ADR) | `vivanta-boot/docs/adr/ADR-NNN-name.md` |
| Аудит, разбор дефектов, разбор кода | `docs/audit/` |
| Эксперимент (активный, с номером) | `docs/experiments/EXP-NNN-name.md` |
| Документация конкретной платы | `docs/hardware/{board}/` |
| Завершённый этап | `docs/milestones/M{n}/acceptance.md` |
| Схема данных/протокола | `docs/architecture/` до появления рабочей схемы |
| Изменения в коде ядра | `vivanta-boot/kernel/src/` |
| Архитектурно-зависимый код | `vivanta-boot/arch-aarch64/` |
| Код платформы | `vivanta-boot/platform-{name}/src/` |
| Программа пользователя (EL0) | `vivanta-boot/user-init/` |
| Скрипт сборки/прошивки/проверки | `vivanta-boot/tools/` |
| История проекта (до-R2) | `docs/history/` — только чтение |
| ZIP-архивы, бинарники | `ai-workstation/Archives/Vivanta/` |
| Личные заметки | Obsidian Vault |

## Где что искать: канонические источники

| Вопрос | Файл-источник |
|--------|---------------|
| Что реально собрано и работает | `vivanta-boot/tools/qemu-gates.sh` (18 маркеров) + `vivanta-boot/kernel/src/lib.rs` |
| Контракт загрузчик ↔ ядро | `vivanta-boot/boot-info/src/lib.rs` + ADR-021 |
| ABI системных вызовов | `vivanta-boot/kernel/src/syscall.rs` + ADR-033 |
| Архитектурные решения | `vivanta-boot/docs/adr/` (18 файлов) |
| Открытые дефекты и блокеры | `docs/audit/2026-09-24-global-audit.md` |
| Что было удалено | `docs/audit/2026-09-25-cleanup.md` |
| Железо RK3568 | `docs/hardware/rk3568/board-info.md` + `uboot-emmc-bringup.md` |
| CI и релиз | `.github/workflows/ci.yml`, `release.yml`, `docs/tooling/ci-cd.md` |

## Конвенции, которые важны

### ADR — единственный канонический журнал решений

ADR лежат в **`vivanta-boot/docs/adr/`**, а не в `docs/adr/`.
Имена: `ADR-NNN-kebab-case-title.md`, номер с ведущими нулями.
Статус (`Accepted` / `Proposed` / `Superseded`) — в теле файла, **не в имени**.

Сейчас существуют ADR-011…015, 017…023, 030…035 — 18 файлов.
**ADR-016, ADR-024 и ADR-025 не существуют.** Если документ на них ссылается —
он неверен.

### `docs/history/` — заморожен, не редактировать

95 файлов, 7.4 МиБ — снимок проекта под прежними именами («Theseus OS»,
«Vorkuta OS»). Не править, не ссылаться как на актуальное, не воскрешать
лежащие там пути. `docs/history/README.md` говорит «not authoritative», но
реально эту пометку несёт 1 файл из 68 `.md` — не рассчитывайте на
самодекларацию.

### `docs/audit/` — источник истины по дефектам

Если код и документация расходятся, расхождение зафиксировано здесь.
`docs/audit/2026-09-24-global-audit.md` — текущий бэклог корректности и
кремниевых блокеров. Всё, что пишется в этот раздел, должно быть
проверяемым по исходникам.

В каталоге ровно **два реальных аудита** — `2026-09-24-global-audit.md` и
`2026-09-25-cleanup.md` — плюс `README.md` в роли индекса.

Осторожно: `docs/history/audit-prompts/Independent_Architecture_Audit.prompt-template.md`
— это **шаблон промпта для внешних LLM без ответов**, а не результат аудита.
Он вынесен под `docs/history/` и переименован, но в старых коммитах встречается
под старым именем — перепутать легко.

### `specs/` — не авторитетно

`specs/` предшествует реорганизации R2 и описывает проект Theseus. Два RFC
про состояние загрузки (`specs/rfc/0003-boot-protocol.md`,
`specs/rfc/008-boot-protocol.md`) независимы друг от друга, и ни один не
совпадает с реально реализованным `BootInfo`. Канонический контракт загрузки —
ADR-021 и `vivanta-boot/docs/architecture/current-architecture.md`.
`specs/schemas/` — два файла, которые **никогда не содержали схемы**
(переписаны 2026-09-29 из 5-строчных `TBD`-заглушек в честные заметки);
машинно-читаемых схем в репозитории нет вообще, а работа по ним заблокирована
scope fence, а не стоит в бэклоге.

### `docs/archive/` ≠ `docs/history/`

`docs/archive/research-track/` — заброшенное направление (дайджест, заметки,
пять больших документов, десять неутверждённых черновиков ADR). Мёртвая
кладовка, не архив.

### Не собираемые каталоги

`vivanta-boot/kernel-memory-frozen/`, `vivanta-boot/user/hello/`,
`vivanta-boot/user/libc/`, `vivanta-boot/archive/boot_legacy/` и
`vivanta-boot/tests/` — вне workspace. Любая cargo-команда внутри них падает с
`current package believes it's in a workspace when it's not`. Это намеренно
(ADR-011: «Frozen systems (archived to RFC, not deleted)»). **Не добавляйте их
в workspace** — это справочный материал, и их код уже не соответствует
контрактам ядра.

### Сборка требует скрипт

`user-init/` — отдельный workspace, а его `user-init.elf` в `.gitignore`
(`.gitignore:13`). Ядро встраивает этот ELF через `include_bytes!`
(`vivanta-boot/kernel/src/syscall/process.rs:545`), поэтому **свежий клон не
соберёт `vivanta-kernel`, пока не отработает**
`vivanta-boot/tools/build-user-init.sh`. Это делают все точки входа:
`build.sh:19-22`, `tools/qemu-gates.sh:52-53`, `.github/workflows/ci.yml:60-64`.

## Что НЕ класть

- ZIP-архивы, бинарники — в `ai-workstation/Archives/Vivanta/`
- `.DS_Store`, `__pycache__/`, `target/` — игнорируются git
- Собранные `.elf`/`.img`/`.bin` — игнорируются git (`.gitignore:12-21`)
- Личные заметки — в Obsidian Vault
- `docs/history/` — не редактировать, это архив
- Скриншоты и дампы логов больших объёмов — в `docs/audit/`, а не в корень

## Гигиена репозитория

443 отслеживаемых файла, `.git` — 4.9 МБ (замерено 2026-09-29). **Самый большой отслеживаемый файл —
минифицированный JavaScript-бандл на 5.6 МБ** в
`docs/history/mindmaps/MindMap/Создание собственной ОС_files/index-86c718b2.js`
(экспорт mindmap из FlowUs, сохранённая целиком веб-страница). На него
приходится ~60% всех байтов репозитория, на каталог `mindmaps/` — ~76%.
Это кандидат на удаление, а не норма: информация продублирована лежащим рядом
`.html` и `.png`.
