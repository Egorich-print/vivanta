// ---------------------------------------------------------------------------
// RunQueue — ready thread queue with priority support
// ---------------------------------------------------------------------------

use super::thread::{Priority, Thread, ThreadId, ThreadState};
use alloc::vec::Vec;

pub struct RunQueue {
    threads: Vec<Option<Thread>>,
    next_id: u64,
}

impl RunQueue {
    pub fn new() -> Self {
        RunQueue {
            threads: Vec::new(),
            next_id: 0,
        }
    }

    /// Allocate a new unique thread ID.
    pub fn alloc_id(&mut self) -> ThreadId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Insert a thread into the runqueue.
    pub fn insert(&mut self, thread: Thread) -> Result<(), RunQueueError> {
        // Find empty slot
        for slot in self.threads.iter_mut() {
            if slot.is_none() {
                *slot = Some(thread);
                return Ok(());
            }
        }
        // No empty slot, push new
        self.threads.push(Some(thread));
        Ok(())
    }

    /// Remove a thread by ID.
    pub fn remove(&mut self, id: ThreadId) -> Option<Thread> {
        for slot in self.threads.iter_mut() {
            if let Some(ref t) = slot {
                if t.id == id {
                    return slot.take();
                }
            }
        }
        None
    }

    /// Get a reference to a thread by ID.
    pub fn get(&self, id: ThreadId) -> Option<&Thread> {
        for slot in self.threads.iter() {
            if let Some(ref t) = slot {
                if t.id == id {
                    return Some(t);
                }
            }
        }
        None
    }

    /// Get a mutable reference to a thread by ID.
    pub fn get_mut(&mut self, id: ThreadId) -> Option<&mut Thread> {
        for slot in self.threads.iter_mut() {
            if let Some(ref mut t) = slot {
                if t.id == id {
                    return Some(t);
                }
            }
        }
        None
    }

    /// Find the next ready thread starting from `from_id`, considering priority.
    pub fn find_next_ready(&self, from_id: ThreadId, exclude_idle: bool) -> Option<ThreadId> {
        let priorities = [
            Priority::Realtime,
            Priority::High,
            Priority::Normal,
            Priority::Low,
        ];

        for priority in priorities {
            // Find starting position
            let start_pos = self
                .threads
                .iter()
                .position(|s| s.as_ref().map_or(false, |t| t.id == from_id))
                .unwrap_or(0);

            // Search forward from that position
            let len = self.threads.len();
            for i in 1..len {
                let idx = (start_pos + i) % len;
                if let Some(ref t) = self.threads[idx] {
                    if t.state == ThreadState::Ready && t.priority == priority {
                        if exclude_idle && t.priority == Priority::Idle {
                            continue;
                        }
                        return Some(t.id);
                    }
                }
            }
        }

        // Fallback to idle if not excluded
        if !exclude_idle {
            for slot in self.threads.iter() {
                if let Some(ref t) = slot {
                    if t.state == ThreadState::Ready && t.priority == Priority::Idle {
                        return Some(t.id);
                    }
                }
            }
        }

        None
    }

    /// Set thread state by ID.
    pub fn set_state(&mut self, id: ThreadId, new_state: ThreadState) -> Result<(), RunQueueError> {
        for slot in self.threads.iter_mut() {
            if let Some(ref mut t) = slot {
                if t.id == id {
                    t.state = new_state;
                    return Ok(());
                }
            }
        }
        Err(RunQueueError::ThreadNotFound)
    }

    /// Iterate over all active threads.
    pub fn iter(&self) -> impl Iterator<Item = &Thread> {
        self.threads.iter().filter_map(|s| s.as_ref())
    }

    /// Iterate over all active threads mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Thread> {
        self.threads.iter_mut().filter_map(|s| s.as_mut())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunQueueError {
    ThreadNotFound,
    QueueFull,
}
