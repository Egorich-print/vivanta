// ---------------------------------------------------------------------------
// Kernel Error Infrastructure
// ---------------------------------------------------------------------------

pub type KernelResult<T> = Result<T, KernelError>;
pub type PmmResult<T> = Result<T, PmmError>;
pub type MmuResult<T> = Result<T, MmuError>;
pub type SchedulerResult<T> = Result<T, SchedulerError>;
pub type MemoryResult<T> = Result<T, MemoryError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    Pmm(PmmError),
    Mmu(MmuError),
    Scheduler(SchedulerError),
    Memory(MemoryError),

    InvalidCapability,
    InvalidAddress,
    DeviceUnavailable,
    InternalInvariant,
}

// ---------------------------------------------------------------------------
// Subsystem Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmmError {
    OutOfMemory,
    InvalidAddress,
    DoubleFree,
    BitmapCorrupted,
    InvalidRegion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmuError {
    TranslationFault,
    PageTableAllocationFailed,
    InvalidMappingFlags,
    AlreadyMapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    RunQueueFull,
    ThreadNotFound,
    InvalidStateTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    AllocationFailed,
    NoPhysAddr,
    MappingFailed,
    UnmappingFailed,
}

// ---------------------------------------------------------------------------
// From Conversions
// ---------------------------------------------------------------------------

impl From<PmmError> for KernelError {
    fn from(err: PmmError) -> Self {
        KernelError::Pmm(err)
    }
}

impl From<MmuError> for KernelError {
    fn from(err: MmuError) -> Self {
        KernelError::Mmu(err)
    }
}

impl From<SchedulerError> for KernelError {
    fn from(err: SchedulerError) -> Self {
        KernelError::Scheduler(err)
    }
}

impl From<MemoryError> for KernelError {
    fn from(err: MemoryError) -> Self {
        KernelError::Memory(err)
    }
}
