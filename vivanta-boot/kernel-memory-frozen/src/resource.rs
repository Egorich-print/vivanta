// ---------------------------------------------------------------------------
// MemoryBackend — abstraction over a heterogeneous memory resource
// ---------------------------------------------------------------------------

/// Physical address returned by a backend allocation.
pub type PhysAddr = u64;

/// Error type for backend allocation failures.
#[derive(Debug, Clone, Copy)]
pub enum AllocError {
    OutOfCapacity,
    AlignmentNotSupported,
    ResourceUnavailable,
}

/// Classification of memory latency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyClass {
    Near,    // e.g. on-package HBM, LLC
    Main,    // e.g. DDR4/5
    Far,     // e.g. CXL-attached, NUMA remote
    Storage, // e.g. SSD, persistent memory
}

/// Classification of memory bandwidth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandwidthClass {
    Extreme, // e.g. HBM (>500 GB/s)
    High,    // e.g. DDR5 (>50 GB/s)
    Medium,  // e.g. DDR4, CXL (>10 GB/s)
    Low,     // e.g. SSD
}

/// Persistence type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceType {
    Volatile,
    Persistent,
}

/// Coherence model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoherenceModel {
    FullyCoherent,
    IoCoherent,
    NonCoherent,
}

/// Reliability class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReliabilityClass {
    Server,   // ECC protected
    Consumer, // no ECC
}

/// Power cost class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerClass {
    Low,
    Medium,
    High,
}

/// Static properties of a memory backend.
#[derive(Debug, Clone, Copy)]
pub struct MemoryProperties {
    pub capacity: usize,
    pub latency_ns: u32,
    pub bandwidth_mb_s: u64,
    pub persistence: PersistenceType,
    pub coherence: CoherenceModel,
    pub reliability: ReliabilityClass,
    pub power: PowerClass,
    pub latency_class: LatencyClass,
    pub bandwidth_class: BandwidthClass,
}

/// Resource identifier — opaque handle used by MRM and MemoryObject.
pub type ResourceId = u64;

/// A backend that provides physical memory pages.
///
/// Each backend corresponds to a single type of hardware memory resource
/// (e.g. system RAM, CXL-attached memory, VRAM, persistent memory).
pub trait MemoryBackend {
    /// Allocate a contiguous region of physical memory.
    fn allocate(&mut self, size: u64, align: u64) -> Result<PhysAddr, AllocError>;

    /// Deallocate a previously allocated region.
    fn deallocate(&mut self, addr: PhysAddr, size: u64);

    /// Return the static properties of this backend.
    fn properties(&self) -> MemoryProperties;

    /// Human-readable name (for diagnostics).
    fn name(&self) -> &'static str;
}
