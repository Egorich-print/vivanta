# ADR-032: User Virtual Memory — Fault Classification, Mapping States and Backing Ownership

## Status
Accepted

## Date
2026-08-21

## Related
ADR-030 (paging mechanism/policy), ADR-031 (VA + page-table ownership),
M5.0 green baseline §G3 (EL0 containment, `elr += 4` prohibition).

---

## 0. Core invariant

> **`MappingSet` is the single authoritative registry of user virtual
> memory. Page tables are a materialization of `MappingSet`, never an
> independent source of state.**

Consequences enforced in this design:

- Backing/lifecycle metadata lives *inside* `Mapping` (INV-VM-002) — there
  is no second registry, fault table or lazy set.
- Only the existing VMM primitives (`map_pages`, `protect`, `unmap_pages`,
  reclamation) may program hardware; the fault resolver is a *client* of
  them, not a second writer (INV-VM-003).
- Every non-Reserved shadow piece has a mechanically checkable hardware
  image: Present ⇔ valid leaf with matching permission bits; Lazy ⇔ no
  leaf (INV-VM-001 verifier).

## 1. Mapping state machine

```rust
pub enum Backing {
    /// Hardware mapping exists for the entire piece.
    Present,
    /// No hardware mapping. First access inside the piece demand-fills
    /// exactly one page from anonymous memory (PMM frame, zeroed).
    LazyAnonymous,
    /// Reservation only; no automatic fill; any access faults fatally.
    Reserved,
}

pub enum PhysOwnership {
    /// PA provided by the caller / MemoryObject — VMM never frees it.
    External,
    /// PA allocated by the VM layer for this piece (anonymous lazy fill);
    /// released to the PMM when the piece is unmapped.
    Anonymous,
}
```

`Mapping` gains `backing: Backing`, `pa: u64` (physical base of Present
pieces; 0 otherwise) and `phys: PhysOwnership`. Shadow pieces are kept
exact by splitting — after a demand fill a Lazy range becomes
`[Lazy head][Present 4K][Lazy tail]`, so piece granularity always equals
hardware granularity and INV-VM-001 stays mechanical.

| state | hw mapping | access → | unmap | protect |
|-------|-----------|----------|-------|---------|
| Reserved | none | fatal | legal (metadata only) | legal (metadata only) |
| LazyAnonymous | none | resolve if access ⊆ perms, else fatal | legal (no frame to free) | legal (metadata only; fills use new perms) |
| Present | whole piece | perm faults fatal | legal (Anonymous ⇒ frame returned) | legal (hardware rewrite) |

## 2. Fault classification

Faults are classified by `(exception class, DFSC/IFSC, access, privilege,
mapping state)`. The authoritative table:

### 2.1 Resolvable (exactly one class in M6.0)

```text
EC  = 0b100101  Data Abort, same EL (EL1)
DFSC ∈ {0b000101, 0b000110, 0b000111}   translation fault L1/L2/L3
access = read or write (WnR = ESR.ISS[6])
active AS = registered AddressSpace matching TTBR0_EL1
mapping = LazyAnonymous piece covering FAR
permission check: write ⇒ permissions.is_read_write()
```
→ demand-fill one page, return from exception, **retry same instruction**.

Everything else that is not listed here or in §2.2 follows the pre-M6.0
paths unchanged.

### 2.2 Always fatal (never resolved, never retried)

- Permission faults (`DFSC 0b001101/0b001110/0b001111`) — hard rule:
  *permission faults are never lazy faults*. A fault against a Present
  piece, an RX page written, NX executed, kernel-only page touched from
  EL0 — all remain containment/fatal paths.
- Instruction aborts (`EC 0b100000/0b100001`) — lazy executable mappings
  are explicitly out of scope for M6.0 (IFSC layout + W^X interplay get
  their own verified step).
- Accesses to `Reserved` pieces.
- Faults where TTBR0 matches no registered address space.
- Faults outside any mapping, or where mapping metadata is inconsistent
  with the request (corrupted state machine).
- OOM during demand fill (§5): logged distinctly, then fatal for EL1.
- All unexpected EC/DFSC combinations at EL1 → pre-existing
  `exception_handler` dump + halt.

### 2.3 EL0 boundary (unchanged)

EL0 sync exceptions keep the M5.0 containment path verbatim: SVC →
syscall dispatch; anything else → task termination. In M6.0 no EL0 code
legitimately holds lazy mappings, so an EL0 fault on a Lazy piece
terminates the task like any other user fault. Resolving EL0-originated
demand fills is future work tied to the syscall ABI.

## 3. Retry semantics — why this is not `elr += 4`

The forbidden pattern advances ELR past the faulting instruction,
*pretending* it executed. Demand-fill retry does the opposite:

1. The faulting instruction did **not** execute; its effect is still
   required.
2. Resolution makes the instruction's access *legal* (descriptor
   installed, memory zero-initialized).
3. The exception returns with **ELR unmodified**; the CPU refetches and
   re-executes the very same instruction, which now succeeds.
4. No TLBI is required for the filled page: the first walk missed (no TLB
   entry was created for an invalid translation), so the retry walks the
   newly installed descriptor. DSB/ISB order the descriptor write before
   the retry.

This is architecturally indistinguishable from what hardware does for
transparent page-table updates, and every resolution is covered by tests
that verify the *effect* of the retried instruction (read/write values),
not just the absence of a panic.

## 4. Ownership

- **Mapping ownership**: `AddressSpace`/`MappingSet` owns the logical
  range and its lifecycle.
- **Physical-frame ownership** is explicit per piece:
  - `External` — caller/MemoryObject owns the PA; VMM never frees it
    (alias policy of ADR-031 preserved);
  - `Anonymous` — the VM layer allocated the frame on demand; the frame
    is reachable only through this mapping; unmapping the piece returns
    the frame to the PMM.
- Aliases of External frames are unaffected by either rule; Anonymous
  frames cannot be aliased because their PA is chosen internally and
  never published.

## 5. OOM semantics

Demand-fill transaction:

```text
validate (state, perms, coverage)
→ allocate PMM frame          ← failure point
→ zero the frame
→ map_pages (single page, current mapping permissions)
→ split shadow: Lazy → [Lazy|Present(Anonymous)|Lazy]
```

Allocation failure leaves the mapping exactly as it was (still Lazy, no
frame, no PTE); the resolver logs `[VM] OOM during demand fill` and
reports fatal. For EL1 the outcome is the standard exception dump +
halt (no recoverable policy exists at boot-monitor stage); for future
EL0 callers this maps to task termination. Table-frame OOM inside
`map_pages` remains boot/runtime-fatal per ADR-031. The transaction is
ordered so the shadow can never claim Present before hardware succeeded
(hard rule #6), and no path frees-or-forgets a frame twice.

## 6. Capacity notes

- `MappingSet` remains fixed at 64 slots in M6.0 (explicit limitation):
  demand-fill splits consume slots, bounded by `MAX_MAPPINGS`; overflow
  fails deterministically before mutation. Heap-backed storage is the
  planned follow-up once user processes need larger mapping counts; it
  must preserve `AddressSpace` size discipline (mission-2 stack lesson).
- `MAX_ADDRESS_SPACES = 8` retained: the fault path identifies the active
  AS by TTBR0 match against registered roots — no ID reuse or stale
  reference is possible; exhaustion panics deterministically at
  registration time.
