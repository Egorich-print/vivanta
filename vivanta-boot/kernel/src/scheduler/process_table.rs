// ---------------------------------------------------------------------------
// Process Table — global registry of all tasks
// ---------------------------------------------------------------------------

use super::task::{MAX_TASKS, ProcessHandle, Task, TaskId, TaskState};
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

    /// Number of live tasks. Reap tombstones (`Exited`) are not live:
    /// counting them turned MAX_TASKS into a lifetime cap — 64
    /// fork/reap cycles would wedge task creation forever.
    pub fn live_count(&self) -> usize {
        use super::task::TaskState;
        self.tasks
            .iter()
            .filter(|s| s.as_ref().is_some_and(|t| t.state != TaskState::Exited))
            .count()
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

        // Find an empty slot — or a reaped tombstone (Exited). Tombstones
        // are resource-free; reusing them keeps the Vec bounded across
        // fork/reap churn (otherwise every cycle pushes ~1 KiB until the
        // kernel heap OOMs). The tombstone's bumped generation is kept so
        // handles to the reaped task stay stale (generation field).
        for slot in self.tasks.iter_mut() {
            let slot_gen = match slot {
                None => 0,
                Some(t) if t.state == TaskState::Exited => t.generation,
                Some(_) => continue,
            };
            task.generation = slot_gen;
            *slot = Some(task);
            return Some(ProcessHandle {
                id: pid,
                generation: slot_gen,
            });
        }
        // No reusable slot, push new
        task.generation = 0;
        self.tasks.push(Some(task));
        Some(ProcessHandle {
            id: pid,
            generation: 0,
        })
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

    /// Count active tasks (same live definition as `live_count`).
    pub fn count(&self) -> usize {
        self.live_count()
    }

    /// Iterate over all tasks.
    pub fn iter(&self) -> impl Iterator<Item = &Task> {
        self.tasks.iter().filter_map(|s| s.as_ref())
    }
}
