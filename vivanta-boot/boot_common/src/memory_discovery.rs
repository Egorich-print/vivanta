// ---------------------------------------------------------------------------
// Memory Discovery — extract usable physical memory regions from BootInfo,
// subtract kernel image, DTB, page tables, and other reserved areas.
//
// Output is a list of `AvailableRegion` suitable for initializing PmmBitmap.
// ---------------------------------------------------------------------------

use crate::MemoryMap;

/// A contiguous range of available physical memory.
#[derive(Debug, Clone, Copy)]
pub struct AvailableRegion {
    pub start: u64,
    pub end: u64, // exclusive
}

/// Maximum number of available regions after splitting/reserving.
pub const MAX_AVAILABLE: usize = 8;

/// Result of memory discovery: free ranges usable by PMM.
pub struct MemoryRegions {
    pub regions: [AvailableRegion; MAX_AVAILABLE],
    pub count: usize,
}

impl MemoryRegions {
    pub fn iter(&self) -> impl Iterator<Item = &AvailableRegion> {
        self.regions[..self.count].iter()
    }
}

/// Information about the kernel image placement.
pub struct KernelLayout {
    pub start: u64, // inclusive
    pub end: u64,   // exclusive
    pub dtb: u64,   // 0 if no DTB
    pub dtb_size: u64,
    pub page_tables_start: u64,
    pub page_tables_size: u64,
}

/// Discover available physical memory regions from the BootInfo memory map.
///
/// Subtracts kernel image, DTB, early page tables, and any reserved/bootloader
/// regions from the usable areas. Returns a sorted, non-overlapping list of
/// free ranges.
pub fn discover(memory_map: &MemoryMap, layout: &KernelLayout) -> MemoryRegions {
    // Collect all reserved intervals
    let mut reserved: [(u64, u64); 32] = [(0, 0); 32];
    let mut reserved_count = 0;

    let mut push_reserved = |start: u64, end: u64| {
        if start < end && reserved_count < reserved.len() {
            reserved[reserved_count] = (start, end);
            reserved_count += 1;
        }
    };

    // Reserved from memory map
    for r in memory_map.regions() {
        match r.kind {
            crate::MemoryRegionKind::Usable => {}
            _ => {
                push_reserved(r.start, r.start + r.size);
            }
        }
    }

    // Kernel image
    push_reserved(layout.start, layout.end);

    // DTB
    if layout.dtb != 0 && layout.dtb_size > 0 {
        push_reserved(layout.dtb, layout.dtb + layout.dtb_size);
    }

    // Early page tables
    if layout.page_tables_size > 0 {
        push_reserved(layout.page_tables_start, layout.page_tables_start + layout.page_tables_size);
    }

    // Sort reserved by start (insertion sort — no_std compatible, n ≤ 32)
    for i in 1..reserved_count {
        let key = reserved[i];
        let mut j = i;
        while j > 0 && reserved[j - 1].0 > key.0 {
            reserved[j] = reserved[j - 1];
            j -= 1;
        }
        reserved[j] = key;
    }

    // Merge overlapping reserved intervals
    let mut merged: [(u64, u64); 32] = [(0, 0); 32];
    let mut merged_count = 0;
    for &(s, e) in reserved[..reserved_count].iter() {
        if merged_count == 0 {
            merged[merged_count] = (s, e);
            merged_count += 1;
        } else {
            let last = &mut merged[merged_count - 1];
            if s <= last.1 {
                last.1 = last.1.max(e);
            } else {
                merged[merged_count] = (s, e);
                merged_count += 1;
            }
        }
    }

    // Subtract reserved from usable regions
    let mut result = MemoryRegions {
        regions: [AvailableRegion { start: 0, end: 0 }; MAX_AVAILABLE],
        count: 0,
    };

    for r in memory_map.regions() {
        if r.kind != crate::MemoryRegionKind::Usable {
            continue;
        }
        let mut cur_start = r.start;
        let region_end = r.start + r.size;

        for &(rs, re) in merged[..merged_count].iter() {
            if rs >= region_end {
                break;
            }
            if re <= cur_start {
                continue;
            }
            // Gap before this reserved range
            if rs > cur_start && result.count < MAX_AVAILABLE {
                result.regions[result.count] = AvailableRegion {
                    start: cur_start,
                    end: rs,
                };
                result.count += 1;
            }
            cur_start = cur_start.max(re);
        }

        // Remaining after last reserved range
        if cur_start < region_end && result.count < MAX_AVAILABLE {
            result.regions[result.count] = AvailableRegion {
                start: cur_start,
                end: region_end,
            };
            result.count += 1;
        }
    }

    result
}
