// ---------------------------------------------------------------------------
// Process Table — global registry of all tasks
// ---------------------------------------------------------------------------

use alloc::vec::Vec;
use super::task::{Task, TaskId, TaskState};

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

    /// Create a new task and return its ID.
    pub fn create(&mut self, mut task: Task) -> TaskId {
        let pid = self.next_pid;
        self.next_pid += 1;
        task.task_id = pid;
        
        // Find empty slot
        for slot in self.tasks.iter_mut() {
            if slot.is_none() {
                *slot = Some(task);
                return pid;
            }
        }
        // No empty slot, push new
        self.tasks.push(Some(task));
        pid
    }

    /// Lookup task by ID.
    pub fn lookup(&self, pid: TaskId) -> Option<&Task> {
        for slot in self.tasks.iter() {
            if let Some(ref t) = slot {
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
            if let Some(ref mut t) = slot {
                if t.task_id == pid {
                    return Some(t);
                }
            }
        }
        None
    }

    /// Remove task by ID.
    pub fn remove(&mut self, pid: TaskId) -> Option<Task> {
        for slot in self.tasks.iter_mut() {
            if let Some(ref t) = slot {
                if t.task_id == pid {
                    return slot.take();
                }
            }
        }
        None
    }

    /// Get all children of a task.
    pub fn children_of(&self, parent: TaskId) -> Vec<TaskId> {
        self.tasks.iter()
            .filter_map(|s| s.as_ref())
            .filter(|t| t.parent == Some(parent))
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
