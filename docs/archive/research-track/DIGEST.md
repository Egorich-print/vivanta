# Research Track Archive — Distributed AI / Heterogeneous Compute (DIGEST)

> Сжатый индекс исследовательского трека (distributed AI / heterogeneous
> compute), датированного 2026-08-09. Полные тексты хранятся рядом в этом
> каталоге. Трек изолирован от инженерного ядра Vivanta (см. master-roadmap:
> "quarantined as distributed/AI research track — no implementation").
>
> ВАЖНО о нумерации: номера ADR-031/032 в этом треке НЕ совпадают с
> инженерными ADR-031 (va-page-table-ownership) и ADR-032 (user-vm-fault-
> policy) в `vivanta-boot/docs/adr/` — это разные деревья документов с
> разной тематикой. Корневой `docs/adr/` — дом для организационных/
> research-ADRs (`docs/adr/README.md`).

## Состав (файлы в этом каталоге)

| Документ | Статус | Суть (1 строка) |
|----------|--------|-----------------|

| `ADR-031-capability-distributed-identity.md` — ADR-031 (research) | Proposed | Кластерная идентичность: Ed25519-ключи узлов, подписанные membership-документы с эпохами; identity workload = node_pk || uuid. |
| ADR-032 (research) | Proposed | Три типа узлов на едином fabric-протоколе: нативный узел Vivanta, управляемый Linux-узел (vivanta-agent), внешний ускоритель. |
| ADR-033 | Proposed | Ресурсы как capability-ориентированные динамические дескрипторы (статический дескриптор + изменяющееся состояние); kernel MemoryObject не затронут. |
| ADR-034 | Proposed | Двухканальный интерфейс ускорителей: общий lifecycle + канал opaque-артефактов + типизированный vendor-extension канал. |
| ADR-035 | Proposed | Workload = capability-notated identity + сериализуемый Execution Graph DAG; целиком user-space, ядро не участвует. |
| ADR-036 | Proposed | Двухуровневое планирование: глобальное размещение через scored cost function + Mesos-style admission (offer/accept). |
| ADR-037 | Proposed | Vivanta Model Package: метаданные/граф/веса/токенизатор; content hash = identity модели; CAS-хранилище. |
| ADR-038 | Proposed | At-least-once исполнение: идемпотентность (workload_id, op_seq), leases с TTL, heartbeats/watchdogs, reconciliation. |
| ADR-039 | Proposed | Переходный vivanta-agent на Linux (AIBOX-1684X): CPU/GPU/NPU через ADR-033/034, llama.cpp/bmrt. |

## Прочее в архиве (актуально)

- `ADR-018-user-entry-transition.root-copy.md` — ранняя копия инженерного
  ADR-018 (авторитетная версия: `vivanta-boot/docs/adr/ADR-018-user-entry-
  transition.md`, отличается только развёрнутым псевдокодом ERET).
- `distributed-suite.md` — дайджест пяти документов distributed-архитектуры
  (`docs/distributed/*.md`, 2026-08-09).
- `evolution-notes.md` — сжатая выжимка evolution-plan / execution-context.

## Нумерация ADR (предупреждение)

Инженерные ADR ядра живут в `vivanta-boot/docs/adr/` и имеют СВОЮ
нумерацию: ADR-031 = va-page-table-ownership, ADR-032 = user-vm-fault-
policy. Номера этого архива (ADR-031…039 research) к ним отношения не
имеют. При добавлении новых инженерных ADR использовать следующее число
после инженерной последовательности, а не после архивной.
