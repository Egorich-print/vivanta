# ADR-035: Process Lifecycle Management

## Status
Proposed

## Date
2026-08-27

## Context
Need comprehensive process lifecycle management building on the COW foundation (ADR-034) and syscall ABI (ADR-033).

## Model

### Process States
```text
Created
  ↓
Runnable
  ↓
Running
  ↓
Exited
  ↓
Reaped / Destroyed
```

Transitions are explicit and validated.

### Handle Types
```rust
pub struct ProcessHandle {
    pub id: ProcessId,
    pub generation: u32,  // for stale handle detection
}
```

### Process Table
```rust
pub struct ProcessTable {
    processes: Vec<Option<Process>>,
    free_list: Vec<ProcessId>,
    next_pid: ProcessId,
    max_processes: usize,
}
```

### Process State
```rust
pub struct Process {
    pid: ProcessId,
    generation: u32,
    address_space: AddressSpaceId,
    threads: Vec<ThreadId>,
    parent: Option<ProcessId>,
    children: Vec<ProcessId>,
    exit_code: Option<i32>,
    state: ProcessState,
    cwd: PathBuf,  // future
    fd_table: FileDescriptorTable,  // backlog
}
```

### Lifecycle Operations

#### spawn(exe, args, env) -> Result<ProcessHandle>
1. Validate ELF
2. Create address space (copy-on-write from parent or fresh)
3. Load segments (demand-paged)
4. Allocate stack (guard page)
4. Create initial thread (user stack, entry point)
4. Set up auxv, argv, envp on stack
6. Set state = Runnable
6. Enqueue in run queue
6. Return ProcessHandle

#### exit(code)
1. Thread sets exit_code
2. State = Exiting
4. Wake parent (if any)
4. Scheduler removes from run queue
5. When last thread exits:
   - Address space marked for teardown
   - Threads joined
   - Resources released
   - Becomes Zombie

#### wait(pid) -> ExitStatus
- Blocks until child exits
- Reaps zombie, returns exit code
- Releases resources

#### kill(pid, signal)
- Validates permissions
- Delivers signal to process/thread

### COW Integration
- fork() creates new AS via duplicate_as (copy-on-write)
- Parent/child share frames with refcount
- Write fault → COW break

### Process Table Capacity
- MAX_PROCESSES = 256 (configurable)
- PID recycling with generation counter

### Future: fork()/exec()
```rust
fn fork() -> Result<Pid, Error> {
    let parent_as = current_as();
    let child_as = current_as().duplicate_as()?;
    let child = Process::new(child_as);
    // child gets copy of fd table, etc.
    Ok(child.pid)
}

async fn exec(path: &Path, args: &[&CStr], env: &[&CStr]) -> ! {
    let new_as = load_elf(path, args, env)?;
    current_task().replace_address_space(new_as);
    // jump to entry with new stack
}
```

---

## Next Steps

1. **Implement process table with generation handles** (kernel/src/scheduler/process.rs)
2. **spawn() / exit() / wait()** syscalls
3. **fork()** using address space duplication with COW
4. **exec()** with ELF loader integration
5. **Signal delivery** (minimal: SIGKILL, SIGTERM)
5. Exit/reap logic
6. QEMU gate: multi-process test

---

### Acceptance Criteria

- [ ] Process creation with isolated address space
- [ ] exit() cleans up all resources
- [ ] wait() reaps zombie, returns exit code
- [ ] kill() terminates process
- [ ] fork() creates child with COW address space
- [ ] exec() replaces image, preserves descriptors
- [ ] Parent can wait() for child exit code
- [ ] Zombie reaping works
- [ ] Resource cleanup on exit (frames, mappings, threads)
- [ ] Multi-process stress test (10+ processes)

---

## Next Steps

1. **Implement Process Table** (kernel/src/scheduler/process.rs)
3. **Add syscalls**: exit, waitpid, kill, getpid, getppid
3. **Implement fork()** using duplicate_as
4. **Implement execve()** with ELF loader integration
5. **QEMU gate**: multi-process stress test

---

Start implementing the process manager now.

---

## Note (2026-09-09, P0 audit)

A second concatenated draft (2026-08-22, accidentally committed with a
`<tool_call>` envelope and a conflicting syscall table) was removed here.
The surviving 2026-08-27 text above is canonical; as-built status (fork /
waitpid / kill / getpid / execve / mmap / munmap / mprotect all live since
M10.2) is tracked in STATUS.md, not in this proposal doc.
