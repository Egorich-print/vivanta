// ---------------------------------------------------------------------------
// Process Table — global registry of all tasks
// ---------------------------------------------------------------------------

use super::task::{MAX_TASKS, ProcessHandle, Task, TaskId};
use alloc::vec::Vec;

/// Global registry of all tasks (M7.4).
///
/// Identity policy: pids are never reused; reaping bumps the slot's
/// generation so `(id, generation)` handles go stale deterministically.
/// Capacity is a hard, deterministic limit (`MAX_TASKS`).
pub struct ProcessTable {
    tasks: Vec<Option<Task>>,
    next_pid: TaskId,
}

impl ProcessTable {
    pub fn new() -> Self {
        ProcessTable {
            tasks: Vec::new(),
            next_pid: 1,
        }
    }

    /// Number of live tasks.
    pub fn live_count(&self) -> usize {
        self.tasks.iter().filter(|s| s.is_some()).count()
    }

    /// Create a new task and return its generation-protected handle.
    /// Fails deterministically at `MAX_TASKS`.
    pub fn create(&mut self, mut task: Task) -> Option<ProcessHandle> {
        if self.live_count() >= MAX_TASKS {
            return None;
        }
        let pid = self.next_pid;
        self.next_pid += 1;
        task.task_id = pid;
        task.generation = 0;

        // Find empty slot
        for slot in self.tasks.iter_mut() {
            if slot.is_none() {
                *slot = Some(task);
                return Some(ProcessHandle {
                    id: pid,
                    generation: 0,
                });
            }
        }
        // No empty slot, push new
        self.tasks.push(Some(task));
        Some(ProcessHandle {
            id: pid,
            generation: 0,
        })
    }

    /// Generation-validated lookup: stale handles resolve to None.
    pub fn lookup_handle(&self, h: ProcessHandle) -> Option<&Task> {
        self.lookup(h.id).filter(|t| t.generation == h.generation)
    }

    pub fn lookup_handle_mut(&mut self, h: ProcessHandle) -> Option<&mut Task> {
        self.lookup_mut(h.id)
            .filter(|t| t.generation == h.generation)
    }

    /// Lookup task by ID.
    pub fn lookup(&self, pid: TaskId) -> Option<&Task> {
        for slot in self.tasks.iter() {
            if let Some(t) = slot {
                if t.task_id == pid {
                    return Some(t);
                }
            }
        }
        None
    }

    /// Lookup task by ID (mutable).
    pub fn lookup_mut(&mut self, pid: TaskId) -> Option<&mut Task> {
        for slot in self.tasks.iter_mut() {
            if let Some(t) = slot {
                if t.task_id == pid {
                    return Some(t);
                }
            }
        }
        None
    }

    /// Remove task by ID and bump its generation so every outstanding
    /// `(id, gen)` handle goes stale immediately. The slot keeps a
    /// resource-free tombstone carrying the new generation.
    pub fn remove(&mut self, pid: TaskId) -> Option<Task> {
        use super::task::TaskState;

        let idx = self
            .tasks
            .iter()
            .position(|s| s.as_ref().is_some_and(|t| t.task_id == pid))?;
        let removed = self.tasks[idx].take()?;
        // Resource-free tombstone carrying the bumped generation.
        // Built field-by-field: Task is not Clone (owns MemoryObjects).
        let mut tomb = Task::new(
            removed.task_id,
            removed.threads.first().copied().unwrap_or(0),
            removed.address_space,
        );
        tomb.generation = removed.generation.wrapping_add(1);
        tomb.state = TaskState::Exited;
        tomb.parent = removed.parent;
        self.tasks[idx] = Some(tomb);
        Some(removed)
    }

    /// Get all children of a task.
    /// Excludes tombstones (Exited) — reaped children are not counted as children.
    /// Zombie children are still returned (not yet reaped).
    pub fn children_of(&self, parent: TaskId) -> Vec<TaskId> {
        self.tasks
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|t| t.parent == Some(parent) && t.state != super::task::TaskState::Exited)
            .map(|t| t.task_id)
            .collect()
    }

    /// Count active tasks.
    pub fn count(&self) -> usize {
        self.tasks.iter().filter(|s| s.is_some()).count()
    }

    /// Iterate over all tasks.
    pub fn iter(&self) -> impl Iterator<Item = &Task> {
        self.tasks.iter().filter_map(|s| s.as_ref())
    }
}
