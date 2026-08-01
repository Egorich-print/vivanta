use vivanta_arch_api::context::{ArchContext, ExecutionLevel};
use crate::vmm::AddressSpaceId;

pub type ThreadId = u64;
pub type ThreadEntry = extern "C" fn(usize);

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
    pub context: ArchContext,
    pub entry: Option<ThreadEntry>,
    pub address_space: AddressSpaceId,
    pub level: ExecutionLevel,
}
