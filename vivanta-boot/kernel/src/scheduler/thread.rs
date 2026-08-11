use crate::vmm::AddressSpaceId;
use vivanta_arch_api::context::{ArchContext, ExecutionLevel};

pub type ThreadId = u64;
pub type ThreadEntry = extern "C" fn(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Realtime = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Idle = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Created,
    Ready,
    Running,
    Blocked,
    Sleeping,
    Terminated,
}

pub struct Thread {
    pub id: ThreadId,
    pub state: ThreadState,
    pub priority: Priority,
    pub context: ArchContext,
    pub entry: Option<ThreadEntry>,
    pub address_space: AddressSpaceId,
    pub level: ExecutionLevel,
    pub sleep_until: Option<u64>, // Tick count when to wake up
    /// Physical base of the contiguous kernel stack (KERNEL_STACK_SIZE).
    /// `None` for the boot/idle threads (static stacks, never freed).
    pub kernel_stack_pa: Option<u64>,
}
