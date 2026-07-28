# Repository Guide

> Как устроен репозиторий Vivanta и куда что класть.

```
Vivanta/
│
├── README.md              ← что это и как начать
├── STATUS.md              ← текущее состояние (коротко)
├── CHANGELOG.md           ← история изменений
│
├── docs/
│   ├── REPOSITORY_GUIDE.md   ← этот файл
│   ├── adr/                  ← архитектурные решения ПОСЛЕ принятия
│   ├── architecture/         ← описание архитектуры, roadmap
│   ├── experiments/          ← временные исследования (active)
│   ├── hardware/             ← документация конкретных плат
│   ├── milestones/           ← завершённые этапы
│   ├── history/              ← архивные документы (не удалять)
│   └── manifesto.md          ← философия проекта
│
├── specs/
│   ├── rfc/                  ← предложения ДО принятия
│   └── schemas/              ← формальные схемы данных
│
├── vivanta-boot/
│   ├── kernel/               ← ядро
│   ├── arch-*/               ← архитектурно-зависимый код
│   ├── platform-*/           ← платформы
│   ├── target-*/             ← бинарные цели
│   ├── boot_common/          ← общий boot-код
│   ├── boot-info/            ← boot information
│   └── tests/                ← тесты
│
└── (archive/ нет — перенесено в docs/history/)
```

## Куда что класть

| Что | Куда |
|-----|------|
| Новое архитектурное решение | `docs/adr/ADR-NNN-name.md` |
| Предложение до утверждения | `specs/rfc/NNN-name.md` |
| Эксперимент/исследование | `docs/experiments/{name}/` |
| Документация платы | `docs/hardware/{platform}/` |
| Завершённый этап | `docs/milestones/M{number}-{name}.md` |
| Временный результат отладки | `docs/experiments/` (потом удалить) |
| Схема данных/протокола | `specs/schemas/{name}.md` |
| История проекта | `docs/history/` (read-only) |
| Исходники ядра | `vivanta-boot/kernel/src/` |
| Код платформы | `vivanta-boot/platform-{name}/src/` |

## Что НЕ класть

- ZIP-архивы, бинарники — в `ai-workstation/Archives/Vivanta/`
- `.DS_Store`, `__pycache__/`, `target/` — игнорируются git
- Личные заметки — в Obsidian Vault
- `docs/history/` — не редактировать, это архив
