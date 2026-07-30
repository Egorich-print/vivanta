use core::fmt;

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
    Near,
    Main,
    Far,
    Storage,
}

/// Classification of memory bandwidth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandwidthClass {
    Extreme,
    High,
    Medium,
    Low,
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
    Server,
    Consumer,
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

impl MemoryProperties {
    pub const fn dram_4gb() -> Self {
        MemoryProperties {
            capacity: (4u64 * 1024 * 1024 * 1024) as usize,
            latency_ns: 80,
            bandwidth_mb_s: 25000,
            persistence: PersistenceType::Volatile,
            coherence: CoherenceModel::FullyCoherent,
            reliability: ReliabilityClass::Consumer,
            power: PowerClass::Medium,
            latency_class: LatencyClass::Main,
            bandwidth_class: BandwidthClass::High,
        }
    }
}

impl fmt::Display for MemoryProperties {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cap={} lat={}ns bw={}MB/s", self.capacity, self.latency_ns, self.bandwidth_mb_s)
    }
}

/// Resource identifier — opaque handle used by MRM and MemoryObject.
pub type ResourceId = u64;

/// A backend that provides physical memory pages.
pub trait MemoryBackend {
    fn allocate(&mut self, size: u64, align: u64) -> Result<PhysAddr, AllocError>;
    fn deallocate(&mut self, addr: PhysAddr, size: u64);
    fn properties(&self) -> MemoryProperties;
    fn name(&self) -> &'static str;
}