// ---------------------------------------------------------------------------
// Policy engine — evaluates backend properties against allocation requirements
// ---------------------------------------------------------------------------

use crate::resource::{
    BandwidthClass, LatencyClass, MemoryProperties, PersistenceType,
};

/// High-level placement preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementPolicy {
    Fastest,
    Largest,
    Persistent,
    Balanced,
}

/// Detailed allocation requirements.
///
/// The policy engine picks the backend whose properties best match these
/// requirements. Fields set to `None` are not constrained.
#[derive(Debug, Clone, Copy)]
pub struct AllocationRequirements {
    pub size: u64,
    pub align: u64,
    pub preferred_policy: PlacementPolicy,

    /// Maximum acceptable latency. `None` = don't care.
    pub max_latency_ns: Option<u32>,
    /// Minimum acceptable bandwidth. `None` = don't care.
    pub min_bandwidth_mb_s: Option<u64>,
    /// If `Some(true)`, only persistent backends qualify.
    pub require_persistence: Option<bool>,
}

impl AllocationRequirements {
    pub const fn new(size: u64) -> Self {
        AllocationRequirements {
            size,
            align: 4096,
            preferred_policy: PlacementPolicy::Balanced,
            max_latency_ns: None,
            min_bandwidth_mb_s: None,
            require_persistence: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

/// Scores are in the range 0..100 per dimension.
struct DimensionScores {
    latency: u32,
    bandwidth: u32,
    capacity: u32,
    persistence: u32,
}

fn score_dimensions(props: &MemoryProperties) -> DimensionScores {
    let latency = match props.latency_class {
        LatencyClass::Near => 100,
        LatencyClass::Main => 60,
        LatencyClass::Far => 20,
        LatencyClass::Storage => 0,
    };
    let bandwidth = match props.bandwidth_class {
        BandwidthClass::Extreme => 100,
        BandwidthClass::High => 70,
        BandwidthClass::Medium => 40,
        BandwidthClass::Low => 10,
    };
    let gib = (props.capacity / (1024 * 1024 * 1024)).max(1);
    let capacity = if gib >= 64 { 100 } else if gib >= 8 { 80 } else if gib >= 1 { 50 } else { 10 };
    let persistence = match props.persistence {
        PersistenceType::Persistent => 100,
        PersistenceType::Volatile => 0,
    };
    DimensionScores { latency, bandwidth, capacity, persistence }
}

/// Weighted sum of dimension scores.
fn weighted_score(dim: &DimensionScores, w: &[u32; 4]) -> u32 {
    (dim.latency * w[0]
        + dim.bandwidth * w[1]
        + dim.capacity * w[2]
        + dim.persistence * w[3])
        / 100
}

/// Compute a single [0..100] score for a backend given the requirements.
pub fn evaluate(props: &MemoryProperties, req: &AllocationRequirements) -> u32 {
    // Hard filters — disqualify if constraints are violated.
    if let Some(max_lat) = req.max_latency_ns {
        if props.latency_ns > max_lat {
            return 0;
        }
    }
    if let Some(min_bw) = req.min_bandwidth_mb_s {
        if props.bandwidth_mb_s < min_bw {
            return 0;
        }
    }
    if let Some(true) = req.require_persistence {
        if props.persistence != PersistenceType::Persistent {
            return 0;
        }
    }
    if props.capacity < req.size as usize {
        return 0;
    }

    let dim = score_dimensions(props);
    match req.preferred_policy {
        PlacementPolicy::Fastest => weighted_score(&dim, &[60, 25, 10, 5]),
        PlacementPolicy::Largest => weighted_score(&dim, &[10, 10, 70, 10]),
        PlacementPolicy::Persistent => weighted_score(&dim, &[10, 10, 20, 60]),
        PlacementPolicy::Balanced => weighted_score(&dim, &[30, 25, 25, 20]),
    }
}

/// Evaluate multiple backends and return the index of the best match.
pub fn select_best<'a>(
    backends: impl Iterator<Item = &'a MemoryProperties>,
    req: &AllocationRequirements,
) -> Option<usize> {
    let mut best: Option<(usize, u32)> = None;
    for (i, props) in backends.enumerate() {
        let score = evaluate(props, req);
        if score == 0 {
            continue;
        }
        if best.map_or(true, |(_, s)| score > s) {
            best = Some((i, score));
        }
    }
    best.map(|(i, _)| i)
}