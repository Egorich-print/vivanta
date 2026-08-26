# ADR-034: Copy-on-Write for Anonymous Private Memory

## Status
Accepted

## Date
2026-08-21

## Related
ADR-031 (page-table ownership), ADR-032 (fault policy), ADR-033 (syscall ABI)

---

## 1. Model

COW вводится как **расширение существующей Mapping state machine**, без
второго реестра (INV-VM-002):

```rust
pub enum PhysOwnership {
    External,          // caller/MemoryObject owns PA — VMM never frees
    Anonymous,         // mapping owns PA; released on unmap
    /// NEW: frame shared by `refcount` mappings via COW.
    /// The LAST owner to unmap receives the physical frame.
    CoWShared { refcount: u32 },
}
```

И `Backing` получает маркер COW:

```rust
pub enum Backing {
    Present,
    LazyAnonymous,
    Reserved,
    /// NEW: Present-эквивалент, но записываемый доступ запрещён до
    /// копирования. Аппаратно всегда RO (или R|X если исходник X).
    CoW,
}
```

**Правило одного источника истины**: refcount живёт в shadow piece
(`PhysOwnership::CoWShared{refcount}`) и является единственным
авторитетным счётчиком. Page tables отражают только факт «страница
доступна на запись».

## 2. Duplication protocol (fork-like)

`duplicate_as(parent) -> child`:

1. Для каждой Parent piece:
   - `Present(External)` → child получает тот же PA, то же permissions,
     но **без write** (write bit снят у обоих: parent и child);
   - `Present(Anonymous)` → то же самое + `CoWShared{refcount+=1}` у обоих;
   - `LazyAnonymous` → child получает независимую Lazy-резервацию того же
     размера/permissions (не shared);
   - `Reserved` → child получает Reserved.
2. Hardware: parent leaf перезаписывается с write=0; child leaf создаётся
   с write=0; TLBI для обоих диапазонов.
3. VA layout ребёнка идентичен родительскому (`va.reserve_at` для каждого
   диапазона).

## 3. Write-fault resolution (CoW classifier)

Расширяется классификация из ADR-032 §2.1:

```text
EC = data abort (same EL or lower EL)
DFSC = permission fault (L1/L2/L3)
access = WRITE
mapping piece = Backing::CoW && PhysOwnership::CoWShared{refcount ≥ 1}
```
→ resolve:
1. allocate new frame (OOM → fatal/false, как в demand-fill);
2. copy old_frame → new_frame (4 KiB через identity mapping);
3. remap THIS AS's page → new_frame, restore original write permission;
4. `refcount -= 1` в ЭТОМ piece;
5. TLBI страницы; retry same instruction.

Если после декремента `refcount == 0`: старый frame возвращается в PMM
(последний держатель не нуждается в копии — но НЕ в этом фолте; это
происходит при unmap последнего владельца).

### Что НЕ является COW fault

Permission fault на обычной Present piece (не CoW) → containment
(terminate/fatal) как раньше. Read-faults на CoW pieces невозможны
(чтение разрешено). Instruction fetch на CoW → fatal.

## 4. Unmap semantics

Unmap CoW piece: `refcount -= 1`. Frame освобождается только когда
refcount достигает 0. Если это последний владелец (refcount==1 до
декремента): frame возвращается в PMM сразу (аналогично Anonymous).

## 5. W^X guarantee

CoW pieces наследуют permissions исходника минус write. Исполнимые CoW
страницы (R|X) остаются R|X — запись в них невозможна и после COW-break.
RW-исходники дают RW после break. RWX невозможно по построению
(decode_prot/plan_load отвергают W+X до COW вступает в игру).

## 6. Limits

- MAX_COW_REFCOUNT = u32::MAX (практически недостижимо)
- COW работает только для anonymous private memory (file-backed — backlog)

## 7. Test matrix (обязательная)

1. parent writes → child sees new value (break happened BEFORE fork? no —
   после дублирования parent пишет первым: parent break, child видит
   СТАРУЮ страницу)
2. child writes → parent unchanged, child changed
3. multiple pages independent copy
4. refcount корректно декрементится при unmap
5. последний unmap освобождает frame
6. read-only CoW (R|X source) — запись фатальна, чтение работает
