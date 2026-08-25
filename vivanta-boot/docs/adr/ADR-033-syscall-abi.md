# ADR-033: User/Kernel Syscall ABI

## Status
Accepted

## Date
2026-08-21

## Related
ADR-032 (fault policy), ADR-031 (page-table ownership), ADR-019 (permissions)

---

## 1. Transport

The boundary uses the native AArch64 SVC mechanism:

```text
EL0: svc #0            → vector lower_aarch64_sync (EC=0b010101)
EL1: el0_sync_handler  → decode x8=num, x0..x5=args
     ↳ syscall_dispatch(root_pa, num, arg0..arg5) -> i64
     ↳ frame.x[0] = result; eret (same instruction AFTER svc resumes)
```

The caller's address space is identified by **TTBR0_EL1 at entry time**
(same rule as the demand-fill path, ADR-032): there is no "current
process" global to go stale.

## 2. Register contract (frozen)

| item | value |
|------|-------|
| syscall number | `x8`, unsigned 64-bit |
| arguments | `x0`–`x5` (six), unsigned 64-bit unless documented otherwise |
| return | `x0`, signed 64-bit: `≥0` success, `<0` `-errno` |
| preserved | `x1`–`x18` are call-clobbered except `x0`; EL0 must not rely on them surviving |
| numbers | stable once shipped; new numbers appended only |
| portability | numbering is Vivanta-specific but conceptually portable; transport (SVC) is AArch64 |

## 3. Numbers (frozen set)

| num | name | status |
|-----|------|--------|
| 0 | READ | reserved, returns `-ENOSYS` (not implemented in M7) |
| 1 | WRITE | implemented (UART console, fd 1/2 only) |
| 2 | EXIT | implemented; terminates calling task with `status=x0 as i32` |
| 3 | YIELD | implemented; scheduling point, returns 0 |
| 4 | MMAP | implemented (M7.2, anonymous lazy only) |
| 5 | MUNMAP | implemented (M7.2) |
| 6 | MPROTECT | implemented (M7.2) |

Unknown number → `-ENOSYS`.

## 4. Error encoding

Linux-compatible negative errno values: `-EPERM=-1, -ENOMEM=-12,
-EFAULT=-14, -EINVAL=-22, -ENOSYS=-38`. Success returns are non-negative
(`mmap` returns the virtual address).

## 5. Pointer / argument validation

- Every user-supplied virtual range is validated against the caller's
  mapping state before use (`MappingSet` lookup + alignment + domain
  bounds). Kernel-range addresses can never be produced by the allocator
  and are rejected by validation.
- Validation happens inside the VMM primitives; the dispatcher performs
  only cheap pre-checks (alignment, zero-length).
- Syscalls do not sleep/block in M7 (single-core, no blocking drivers);
  every call either completes or fails deterministically.

## 6. Per-call semantics (frozen for M7)

### EXIT(status:i32)
Terminates the calling task through the existing lifecycle
(thread_exit→task Zombie→reap releases mappings, anonymous frames and
reclaimable tables). Never returns to EL0.

### YIELD()
Scheduler scheduling point. Returns 0.

### MMAP(addr, len, prot, _flags=0, _fd=-1, _off=0)
- `addr`: hint; **must be 0** (fixed mappings unsupported in M7) else `-EINVAL`.
- `len`: >0, ≤ 64 MiB (`MAX_MMAP_BYTES`), rounded up to page.
- `prot`: bit0=R (required), bit1=W, bit2=X; `W&&X` → `-EPERM` (W^X);
  unknown bits → `-EINVAL`.
- Always creates an **anonymous LazyAnonymous** reservation backed by the
  proven demand-fill path; no physical frames are touched at call time.
- Overlap policy: the allocator guarantees disjoint ranges; overlap is
  impossible by construction.
- Returns the base virtual address.
- Limitation (explicit): file-backed/fixed mappings do not exist in M7.

### MUNMAP(addr, len)
- `addr` page-aligned and within the user domain else `-EINVAL`;
  `len>0` else `-EINVAL`; range must intersect a mapping else `-ENOMEM`.
- Partial unmapping splits shadow pieces; fully-covered Present
  Anonymous frames are returned to the PMM; tables are reclaimed when
  provably empty (ADR-031). Returns 0.

### MPROTECT(addr, len, prot)
- Same prot rules as MMAP (`W&&X → -EPERM`); range must be covered by
  mappings else `-ENOMEM`.
- Present pieces: hardware reprogrammed (TLB discipline per tlbi_range);
  Lazy pieces: metadata-only, fills use the new permissions.
- Returns 0.

### READ(fd,buf,n)
Reserved: returns `-ENOSYS` in M7.
