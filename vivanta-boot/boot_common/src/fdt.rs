// ---------------------------------------------------------------------------
// Flattened Device Tree scanner — shared for all boot adapters
// ---------------------------------------------------------------------------

use crate::hardware::{HardwareNode, InterruptControllerInfo, MmioRegion};
use crate::{print, println, MemoryMap, MemoryRegion, MemoryRegionKind};

const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32 = 0x00000002;
const FDT_PROP: u32 = 0x00000003;
const FDT_NOP: u32 = 0x00000004;
const FDT_END: u32 = 0x00000009;

unsafe fn read_be_u32(ptr: *const u8) -> u32 {
    let bytes = core::slice::from_raw_parts(ptr, 4);
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

unsafe fn read_be_u64(ptr: *const u8, ncells: u32) -> u64 {
    let mut val: u64 = 0;
    for i in 0..ncells {
        let cell = read_be_u32(ptr.add(i as usize * 4));
        val = (val << 32) | (cell as u64);
    }
    val
}

fn align4(offset: usize) -> usize {
    (offset + 3) & !3
}

/// Parse a DTB and extract usable memory regions, CPU count, and console info.
pub struct FdtScanner;

impl FdtScanner {
    /// Validate magic, print diagnostics, fill `mem` with usable regions.
    pub unsafe fn probe(dtb_addr: *const u8, mem: &mut MemoryMap) -> bool {
        if dtb_addr.is_null() {
            println!("FDT: null pointer");
            return false;
        }

        let magic = read_be_u32(dtb_addr);
        if magic != 0xD00DFEED {
            println!("FDT: bad magic 0x{:08X}", magic);
            return false;
        }

        let totalsize = read_be_u32(dtb_addr.add(4));
        let off_dt_struct = read_be_u32(dtb_addr.add(8));
        let off_dt_strings = read_be_u32(dtb_addr.add(12));
        let version = read_be_u32(dtb_addr.add(20));
        let size_dt_struct = read_be_u32(dtb_addr.add(36));

        println!(
            "FDT: magic OK, version {}, total {} bytes",
            version, totalsize
        );

        if version < 16 {
            println!("FDT: version too old");
            return false;
        }

        let struct_addr = dtb_addr.add(off_dt_struct as usize);
        let struct_end = struct_addr.add(size_dt_struct as usize);
        let strings_addr = dtb_addr.add(off_dt_strings as usize);

        let mut addr_cells: [u32; 8] = [2; 8];
        let mut size_cells: [u32; 8] = [2; 8];
        let mut depth: usize = 0;

        let mut pos = struct_addr;
        let mut current_name: &'static str = "";

        loop {
            if pos.add(4) > struct_end {
                break;
            }
            let token = read_be_u32(pos);
            pos = pos.add(4);

            match token {
                FDT_BEGIN_NODE => {
                    let name_start = pos;
                    let mut name_len = 0;
                    while *name_start.add(name_len) != 0 {
                        name_len += 1;
                    }
                    let name_bytes = core::slice::from_raw_parts(name_start, name_len);
                    current_name = core::str::from_utf8(name_bytes).unwrap_or("?");
                    pos = pos.add(name_len + 1);
                    let padding = (4 - (pos as usize - 4) % 4) % 4;
                    pos = pos.add(padding);

                    depth += 1;
                    if depth < 8 {
                        addr_cells[depth] = addr_cells[depth - 1];
                        size_cells[depth] = size_cells[depth - 1];
                    }
                }

                FDT_END_NODE => {
                    depth -= 1;
                }

                FDT_PROP => {
                    if pos.add(8) > struct_end {
                        break;
                    }
                    let prop_len = read_be_u32(pos) as usize;
                    let name_off = read_be_u32(pos.add(4)) as usize;
                    pos = pos.add(8);

                    let name_ptr = strings_addr.add(name_off);
                    let mut pname_len = 0;
                    while *name_ptr.add(pname_len) != 0 {
                        pname_len += 1;
                    }
                    let pname_bytes = core::slice::from_raw_parts(name_ptr, pname_len);
                    let pname = core::str::from_utf8(pname_bytes).unwrap_or("?");

                    let value_ptr = pos;
                    let aligned_len = align4(prop_len);
                    pos = pos.add(aligned_len);

                    if pname == "#address-cells" && prop_len >= 4 {
                        let v = read_be_u32(value_ptr);
                        if depth < 8 {
                            addr_cells[depth] = v;
                        }
                        continue;
                    }
                    if pname == "#size-cells" && prop_len >= 4 {
                        let v = read_be_u32(value_ptr);
                        if depth < 8 {
                            size_cells[depth] = v;
                        }
                        continue;
                    }

                    if pname == "reg" && current_name.starts_with("memory") {
                        let parent_ac = if depth > 1 && depth - 1 < 8 {
                            addr_cells[depth - 1]
                        } else {
                            2
                        };
                        let parent_sc = if depth > 1 && depth - 1 < 8 {
                            size_cells[depth - 1]
                        } else {
                            2
                        };
                        let entry_bytes = ((parent_ac + parent_sc) * 4) as usize;
                        let n_entries = if entry_bytes > 0 {
                            prop_len / entry_bytes
                        } else {
                            0
                        };
                        for i in 0..n_entries {
                            let off = i * entry_bytes;
                            let base = read_be_u64(value_ptr.add(off), parent_ac);
                            let size =
                                read_be_u64(value_ptr.add(off + parent_ac as usize * 4), parent_sc);
                            mem.push(MemoryRegion {
                                start: base,
                                size,
                                kind: MemoryRegionKind::Usable,
                            });
                            println!(
                                "  FDT:   RAM 0x{:016x} – 0x{:016x}  ({} MiB)",
                                base,
                                base + size - 1,
                                size >> 20,
                            );
                        }
                        continue;
                    }

                    let mut vlen = prop_len;
                    if vlen > 0 && *value_ptr.add(vlen - 1) == 0 {
                        vlen -= 1;
                    }
                    let value_bytes = core::slice::from_raw_parts(value_ptr, vlen);
                    let value_str = core::str::from_utf8(value_bytes).unwrap_or("<binary>");

                    let indent = if depth <= 1 { "" } else { "  " };
                    match pname {
                        "model" | "compatible" => {
                            println!("  FDT: {} {} = {}", indent, current_name, value_str);
                        }
                        "serial-number" => {
                            println!("  FDT: {} {} → serial: {}", indent, current_name, value_str);
                        }
                        "stdout-path" => {
                            println!("  FDT: {} console: {}", indent, value_str);
                        }
                        "device_type" if value_str == "memory" => {
                            println!("  FDT: {} memory node found", indent);
                        }
                        _ => {}
                    }
                }

                FDT_NOP => {}

                FDT_END => break,

                _ => {
                    println!("FDT: unknown token 0x{:08X}", token);
                    break;
                }
            }
        }

        println!(
            "FDT: scan complete – {} memory region(s)",
            mem.regions().len()
        );
        true
    }

    /// Locate the stdout-path console node and return its HardwareNode.
    pub unsafe fn console(dtb_addr: *const u8) -> Option<HardwareNode> {
        let off_dt_struct = read_be_u32(dtb_addr.add(8));
        let off_dt_strings = read_be_u32(dtb_addr.add(12));
        let size_dt_struct = read_be_u32(dtb_addr.add(36));

        let struct_addr = dtb_addr.add(off_dt_struct as usize);
        let struct_end = struct_addr.add(size_dt_struct as usize);
        let strings_addr = dtb_addr.add(off_dt_strings as usize);

        // Pass 1: find stdout-path in /chosen
        let mut pos = struct_addr;
        let mut depth = 0;
        let mut in_chosen = false;
        let mut current_name;
        let mut target_node: Option<&'static str> = None;

        loop {
            if pos.add(4) > struct_end {
                break;
            }
            let token = read_be_u32(pos);
            pos = pos.add(4);

            match token {
                FDT_BEGIN_NODE => {
                    let name_start = pos;
                    let mut name_len = 0;
                    while *name_start.add(name_len) != 0 {
                        name_len += 1;
                    }
                    let name_bytes = core::slice::from_raw_parts(name_start, name_len);
                    current_name = core::str::from_utf8(name_bytes).unwrap_or("?");
                    pos = pos.add(name_len + 1);
                    let padding = (4 - (pos as usize - 4) % 4) % 4;
                    pos = pos.add(padding);
                    depth += 1;
                    if current_name == "chosen" || current_name == "/chosen" {
                        in_chosen = true;
                    }
                }

                FDT_END_NODE => {
                    if in_chosen && depth == 1 {
                        break;
                    }
                    depth -= 1;
                }

                FDT_PROP => {
                    if pos.add(8) > struct_end {
                        break;
                    }
                    let prop_len = read_be_u32(pos) as usize;
                    let name_off = read_be_u32(pos.add(4)) as usize;
                    pos = pos.add(8);

                    let name_ptr = strings_addr.add(name_off);
                    let mut pname_len = 0;
                    while *name_ptr.add(pname_len) != 0 {
                        pname_len += 1;
                    }
                    let pname_bytes = core::slice::from_raw_parts(name_ptr, pname_len);
                    let pname = core::str::from_utf8(pname_bytes).unwrap_or("?");
                    let aligned_len = align4(prop_len);
                    let value_ptr = pos;
                    pos = pos.add(aligned_len);

                    if in_chosen && pname == "stdout-path" {
                        let mut vlen = prop_len;
                        if vlen > 0 && *value_ptr.add(vlen - 1) == 0 {
                            vlen -= 1;
                        }
                        let path_bytes = core::slice::from_raw_parts(value_ptr, vlen);
                        let path = core::str::from_utf8(path_bytes).unwrap_or("");
                        // Strip options after ':' and take last path component
                        let path = if let Some(col) = path.find(':') {
                            &path[..col]
                        } else {
                            path
                        };
                        // Extract the node name (last path component)
                        target_node = path.rsplit('/').next();
                    }
                }

                FDT_NOP => {}
                FDT_END => break,
                _ => break,
            }
        }

        let target = target_node?;
        if target.is_empty() {
            return None;
        }

        // Pass 2: find a BEGIN_NODE whose name matches `target`
        let mut result = HardwareNode::empty();
        let mut pos = struct_addr;
        let mut depth = 0;
        let mut found = false;

        loop {
            if pos.add(4) > struct_end {
                break;
            }
            let token = read_be_u32(pos);
            pos = pos.add(4);

            match token {
                FDT_BEGIN_NODE => {
                    let name_start = pos;
                    let mut name_len = 0;
                    while *name_start.add(name_len) != 0 {
                        name_len += 1;
                    }
                    let name_bytes = core::slice::from_raw_parts(name_start, name_len);
                    current_name = core::str::from_utf8(name_bytes).unwrap_or("?");
                    pos = pos.add(name_len + 1);
                    let padding = (4 - (pos as usize - 4) % 4) % 4;
                    pos = pos.add(padding);
                    depth += 1;

                    if !found && current_name.starts_with(target) {
                        found = true;
                    }
                }

                FDT_END_NODE => {
                    if found && depth <= 1 {
                        break;
                    }
                    depth -= 1;
                }

                FDT_PROP => {
                    if pos.add(8) > struct_end {
                        break;
                    }
                    let prop_len = read_be_u32(pos) as usize;
                    let name_off = read_be_u32(pos.add(4)) as usize;
                    pos = pos.add(8);

                    let name_ptr = strings_addr.add(name_off);
                    let mut pname_len = 0;
                    while *name_ptr.add(pname_len) != 0 {
                        pname_len += 1;
                    }
                    let pname_bytes = core::slice::from_raw_parts(name_ptr, pname_len);
                    let pname = core::str::from_utf8(pname_bytes).unwrap_or("?");
                    let aligned_len = align4(prop_len);
                    let value_ptr = pos;
                    pos = pos.add(aligned_len);

                    if found {
                        match pname {
                            "compatible" if result.compatible.is_empty() => {
                                let mut vlen = prop_len;
                                if vlen > 0 && *value_ptr.add(vlen - 1) == 0 {
                                    vlen -= 1;
                                }
                                let bytes = core::slice::from_raw_parts(value_ptr, vlen);
                                let first = bytes.split(|&b| b == 0).next().unwrap_or(b"");
                                let s = core::str::from_utf8(first).unwrap_or("");
                                result.compatible = s;
                            }
                            "reg" if result.reg.is_none() && prop_len >= 8 => {
                                let addr = read_be_u64(value_ptr, 2);
                                let size = read_be_u64(value_ptr.add(8), 2);
                                result.reg = Some(MmioRegion { addr, size });
                            }
                            "interrupts" if prop_len >= 4 => {
                                result.irq = Some(read_be_u32(value_ptr));
                            }
                            "reg-shift" if prop_len >= 4 => {
                                result.reg_shift = Some(read_be_u32(value_ptr));
                            }
                            "reg-io-width" if prop_len >= 4 => {
                                result.reg_io_width = Some(read_be_u32(value_ptr));
                            }
                            "clock-frequency" if prop_len >= 4 => {
                                result.clock_frequency = Some(read_be_u32(value_ptr));
                            }
                            "current-speed" if prop_len >= 4 => {
                                result.current_speed = Some(read_be_u32(value_ptr));
                            }
                            _ => {}
                        }
                    }
                }

                FDT_NOP => {}
                FDT_END => break,
                _ => break,
            }
        }

        if result.compatible.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Walk the FDT once, printing a structured validation report while filling
    /// the MemoryMap with usable regions. Returns the number of CPU cores found.
    ///
    /// This is the Stage 1.5 diagnostic entry point — it replaces `probe()` in
    /// adapters that want a comprehensive dump alongside memory discovery.
    ///
    /// Call AFTER `console()` + `init_console()` so `println!` is available.
    pub unsafe fn report(dtb_addr: *const u8, mem: &mut MemoryMap) -> usize {
        if dtb_addr.is_null() {
            println!("FDT: null pointer");
            return 0;
        }

        let magic = read_be_u32(dtb_addr);
        if magic != 0xD00DFEED {
            println!("FDT: bad magic 0x{:08X}", magic);
            return 0;
        }

        let totalsize = read_be_u32(dtb_addr.add(4));
        let off_dt_struct = read_be_u32(dtb_addr.add(8));
        let off_dt_strings = read_be_u32(dtb_addr.add(12));
        let version = read_be_u32(dtb_addr.add(20));
        let size_dt_struct = read_be_u32(dtb_addr.add(36));

        let struct_addr = dtb_addr.add(off_dt_struct as usize);
        let struct_end = struct_addr.add(size_dt_struct as usize);
        let strings_addr = dtb_addr.add(off_dt_strings as usize);

        // State accumulated during the single pass
        let mut cpu_count = 0;
        let mut cpu_header_printed = false;
        let mut depth: usize = 0;
        let mut current_name: &str = "";
        let mut addr_cells: [u32; 8] = [2; 8];
        let mut size_cells: [u32; 8] = [2; 8];

        println!();
        println!("FDT Validation Report");
        println!("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
        println!("  magic:      0x{:08X}", magic);
        println!("  version:    {}", version);
        println!("  totalsize:  {} bytes", totalsize);

        let mut pos = struct_addr;
        loop {
            if pos.add(4) > struct_end {
                break;
            }
            let token = read_be_u32(pos);
            pos = pos.add(4);

            match token {
                FDT_BEGIN_NODE => {
                    let name_start = pos;
                    let mut name_len = 0;
                    while *name_start.add(name_len) != 0 {
                        name_len += 1;
                    }
                    let name_bytes = core::slice::from_raw_parts(name_start, name_len);
                    current_name = core::str::from_utf8(name_bytes).unwrap_or("?");
                    pos = pos.add(name_len + 1);
                    let padding = (4 - (pos as usize - 4) % 4) % 4;
                    pos = pos.add(padding);

                    depth += 1;
                    if depth < 8 {
                        addr_cells[depth] = addr_cells[depth - 1];
                        size_cells[depth] = size_cells[depth - 1];
                    }
                }

                FDT_END_NODE => {
                    depth -= 1;
                }

                FDT_PROP => {
                    if pos.add(8) > struct_end {
                        break;
                    }
                    let prop_len = read_be_u32(pos) as usize;
                    let name_off = read_be_u32(pos.add(4)) as usize;
                    pos = pos.add(8);

                    let name_ptr = strings_addr.add(name_off);
                    let mut pname_len = 0;
                    while *name_ptr.add(pname_len) != 0 {
                        pname_len += 1;
                    }
                    let pname_bytes = core::slice::from_raw_parts(name_ptr, pname_len);
                    let pname = core::str::from_utf8(pname_bytes).unwrap_or("?");

                    let value_ptr = pos;
                    let aligned_len = align4(prop_len);
                    pos = pos.add(aligned_len);

                    // #address-cells / #size-cells at current node
                    if pname == "#address-cells" && prop_len >= 4 {
                        let v = read_be_u32(value_ptr);
                        if depth < 8 {
                            addr_cells[depth] = v;
                        }
                        continue;
                    }
                    if pname == "#size-cells" && prop_len >= 4 {
                        let v = read_be_u32(value_ptr);
                        if depth < 8 {
                            size_cells[depth] = v;
                        }
                        continue;
                    }

                    // Root-level model + compatible
                    if depth == 1 {
                        if pname == "model" {
                            let mut vlen = prop_len;
                            if vlen > 0 && *value_ptr.add(vlen - 1) == 0 {
                                vlen -= 1;
                            }
                            let bytes = core::slice::from_raw_parts(value_ptr, vlen);
                            let s = core::str::from_utf8(bytes).unwrap_or("?");
                            println!("  model:      {}", s);
                        }
                        if pname == "compatible" {
                            let mut vlen = prop_len;
                            if vlen > 0 && *value_ptr.add(vlen - 1) == 0 {
                                vlen -= 1;
                            }
                            let bytes = core::slice::from_raw_parts(value_ptr, vlen);
                            let s = core::str::from_utf8(bytes).unwrap_or("?");
                            print!("  compatible:");
                            for compat in s.split('\0') {
                                print!(" {}", compat);
                            }
                            println!();
                        }
                    }

                    // CPU detection via device_type
                    if pname == "device_type" && prop_len <= 16 {
                        let bytes = core::slice::from_raw_parts(value_ptr, prop_len);
                        if let Ok(s) = core::str::from_utf8(bytes) {
                            let trimmed = s.trim_end_matches('\0');
                            if trimmed == "cpu" {
                                cpu_count += 1;
                                if !cpu_header_printed {
                                    println!("  CPU:");
                                    cpu_header_printed = true;
                                }
                            }
                        }
                    }

                    if pname == "reg" && current_name.starts_with("memory") {
                        let parent_ac = if depth > 1 && depth - 1 < 8 {
                            addr_cells[depth - 1]
                        } else {
                            2
                        };
                        let parent_sc = if depth > 1 && depth - 1 < 8 {
                            size_cells[depth - 1]
                        } else {
                            2
                        };
                        let entry_bytes = ((parent_ac + parent_sc) * 4) as usize;
                        let n_entries = if entry_bytes > 0 {
                            prop_len / entry_bytes
                        } else {
                            0
                        };
                        for i in 0..n_entries {
                            let off = i * entry_bytes;
                            let base = read_be_u64(value_ptr.add(off), parent_ac);
                            let size =
                                read_be_u64(value_ptr.add(off + parent_ac as usize * 4), parent_sc);
                            mem.push(MemoryRegion {
                                start: base,
                                size,
                                kind: MemoryRegionKind::Usable,
                            });
                            println!(
                                "  memory@0x{:x}: 0x{:016x} – 0x{:016x}  ({} MiB)",
                                i,
                                base,
                                base + size - 1,
                                size >> 20,
                            );
                        }
                        continue;
                    }

                    if pname == "compatible" && current_name.starts_with("cpu") && cpu_count > 0 {
                        // CPU compatible — format is "arm,cortex-a53\0..."
                        let mut vlen = prop_len;
                        if vlen > 0 && *value_ptr.add(vlen - 1) == 0 {
                            vlen -= 1;
                        }
                        let bytes = core::slice::from_raw_parts(value_ptr, vlen);
                        let compat = core::str::from_utf8(bytes).unwrap_or("?");
                        let first = compat.split('\0').next().unwrap_or(compat);
                        println!("    cpu{}: compatible = {}", cpu_count - 1, first);
                    }
                }

                FDT_NOP => {}
                FDT_END => break,
                _ => {
                    println!("FDT: unknown token 0x{:08X}", token);
                    break;
                }
            }
        }

        println!("  cpus:       {}", cpu_count);
        println!("  regions:    {} (usable memory)", mem.regions().len());
        println!("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");

        cpu_count
    }

    /// Return the number of CPU cores in the DTB.
    ///
    /// Counts nodes whose name starts with `cpu` AND whose `device_type` is
    /// `"cpu"`.  This handles both hierarchical DTBs (`/cpus/cpu@0`) and
    /// flat DTBs (`/cpu@0` at root) without overcounting containers like
    /// `cpu-map`.
    pub unsafe fn cpu_count(dtb_addr: *const u8) -> usize {
        let off_dt_struct = read_be_u32(dtb_addr.add(8));
        let off_dt_strings = read_be_u32(dtb_addr.add(12));
        let size_dt_struct = read_be_u32(dtb_addr.add(36));

        let struct_addr = dtb_addr.add(off_dt_struct as usize);
        let struct_end = struct_addr.add(size_dt_struct as usize);
        let strings_addr = dtb_addr.add(off_dt_strings as usize);

        let mut pos = struct_addr;
        let mut count = 0;
        // Track whether the current node is actually a CPU core.
        // Starts false for each BEGIN_NODE; set true if device_type = "cpu"
        // is seen before any child node.
        let mut pending_is_cpu = false;

        loop {
            if pos.add(4) > struct_end {
                break;
            }
            let token = read_be_u32(pos);
            pos = pos.add(4);

            match token {
                FDT_BEGIN_NODE => {
                    let name_start = pos;
                    let mut name_len = 0;
                    while *name_start.add(name_len) != 0 {
                        name_len += 1;
                    }
                    let name_bytes = core::slice::from_raw_parts(name_start, name_len);
                    let _node_name = core::str::from_utf8(name_bytes).unwrap_or("?");
                    pos = pos.add(name_len + 1);
                    let padding = (4 - (pos as usize - 4) % 4) % 4;
                    pos = pos.add(padding);

                    pending_is_cpu = false;
                }

                FDT_END_NODE => {
                    if pending_is_cpu {
                        count += 1;
                        pending_is_cpu = false;
                    }
                }

                FDT_PROP => {
                    if pos.add(8) > struct_end {
                        break;
                    }
                    let prop_len = read_be_u32(pos) as usize;
                    let name_off = read_be_u32(pos.add(4)) as usize;
                    pos = pos.add(8);

                    let name_ptr = strings_addr.add(name_off);
                    let mut pname_len = 0;
                    while *name_ptr.add(pname_len) != 0 {
                        pname_len += 1;
                    }
                    let pname_bytes = core::slice::from_raw_parts(name_ptr, pname_len);
                    let pname = core::str::from_utf8(pname_bytes).unwrap_or("?");

                    let value_ptr = pos;
                    let aligned_len = align4(prop_len);
                    pos = pos.add(aligned_len);

                    if pname == "device_type" {
                        let mut vlen = prop_len;
                        if vlen > 0 && *value_ptr.add(vlen - 1) == 0 {
                            vlen -= 1;
                        }
                        let bytes = core::slice::from_raw_parts(value_ptr, vlen);
                        let val = core::str::from_utf8(bytes).unwrap_or("");
                        pending_is_cpu = val == "cpu";
                    }
                }

                FDT_NOP => {}
                FDT_END => break,
                _ => break,
            }
        }

        count
    }

    /// Locate the first interrupt controller node in the FDT and return its
    /// MMIO regions and compatible string.
    ///
    /// Matches nodes whose `compatible` contains "arm,cortex-a15-gic"
    /// (GICv2) or "arm,gic-v3" (GICv3).
    pub unsafe fn interrupt_controller(dtb_addr: *const u8) -> Option<InterruptControllerInfo> {
        let off_dt_struct = read_be_u32(dtb_addr.add(8));
        let off_dt_strings = read_be_u32(dtb_addr.add(12));
        let size_dt_struct = read_be_u32(dtb_addr.add(36));

        let struct_addr = dtb_addr.add(off_dt_struct as usize);
        let struct_end = struct_addr.add(size_dt_struct as usize);
        let strings_addr = dtb_addr.add(off_dt_strings as usize);

        // Pass 1: find the depth of the GIC node by scanning compatible
        let mut target_depth = usize::MAX;
        let mut depth = 0;
        let mut pos = struct_addr;

        loop {
            if pos.add(4) > struct_end {
                break;
            }
            let token = read_be_u32(pos);
            pos = pos.add(4);

            match token {
                FDT_BEGIN_NODE => {
                    let name_start = pos;
                    let mut name_len = 0;
                    while *name_start.add(name_len) != 0 {
                        name_len += 1;
                    }
                    pos = pos.add(name_len + 1);
                    let padding = (4 - (pos as usize - 4) % 4) % 4;
                    pos = pos.add(padding);
                    depth += 1;
                }

                FDT_END_NODE => {
                    if depth == target_depth {
                        break;
                    }
                    depth -= 1;
                }

                FDT_PROP => {
                    if pos.add(8) > struct_end {
                        break;
                    }
                    let prop_len = read_be_u32(pos) as usize;
                    let name_off = read_be_u32(pos.add(4)) as usize;
                    pos = pos.add(8);

                    let name_ptr = strings_addr.add(name_off);
                    let mut pname_len = 0;
                    while *name_ptr.add(pname_len) != 0 {
                        pname_len += 1;
                    }
                    let pname_bytes = core::slice::from_raw_parts(name_ptr, pname_len);
                    let pname = core::str::from_utf8(pname_bytes).unwrap_or("?");

                    let aligned_len = align4(prop_len);
                    let value_ptr = pos;
                    pos = pos.add(aligned_len);

                    if target_depth == usize::MAX && pname == "compatible" {
                        let mut vlen = prop_len;
                        if vlen > 0 && *value_ptr.add(vlen - 1) == 0 {
                            vlen -= 1;
                        }
                        let bytes = core::slice::from_raw_parts(value_ptr, vlen);
                        if let Ok(s) = core::str::from_utf8(bytes) {
                            if s.contains("cortex-a15-gic") || s.contains("gic-v3") {
                                target_depth = depth;
                            }
                        }
                    }
                }

                FDT_NOP => {}
                FDT_END => break,
                _ => break,
            }
        }

        if target_depth == usize::MAX {
            return None;
        }

        // Pass 2: extract compatible + reg from the target node
        // Uses a `found_gic` flag so we only break when exiting the GIC node,
        // not any sibling node at the same depth.
        let mut pos = struct_addr;
        depth = 0;
        let mut addr_cells: [u32; 8] = [2; 8];
        let mut size_cells: [u32; 8] = [2; 8];
        let mut compatible: &str = "";
        let mut distributor: Option<MmioRegion> = None;
        let mut redistributor: Option<MmioRegion> = None;
        let mut found_gic = false;
        let mut inside_gic = false;

        loop {
            if pos.add(4) > struct_end {
                break;
            }
            let token = read_be_u32(pos);
            pos = pos.add(4);

            match token {
                FDT_BEGIN_NODE => {
                    let name_start = pos;
                    let mut name_len = 0;
                    while *name_start.add(name_len) != 0 {
                        name_len += 1;
                    }
                    pos = pos.add(name_len + 1);
                    let padding = (4 - (pos as usize - 4) % 4) % 4;
                    pos = pos.add(padding);
                    depth += 1;
                    if depth < 8 {
                        addr_cells[depth] = addr_cells[depth - 1];
                        size_cells[depth] = size_cells[depth - 1];
                    }
                    // Start "inside" when we reach target depth level
                    if !inside_gic && depth == target_depth {
                        inside_gic = true;
                    }
                }

                FDT_END_NODE => {
                    if inside_gic && depth == target_depth {
                        if found_gic {
                            // Exiting the GIC node — done
                            break;
                        }
                        // Exiting some other node — keep looking
                        inside_gic = false;
                    }
                    depth -= 1;
                }

                FDT_PROP => {
                    if pos.add(8) > struct_end {
                        break;
                    }
                    let prop_len = read_be_u32(pos) as usize;
                    let name_off = read_be_u32(pos.add(4)) as usize;
                    pos = pos.add(8);

                    let name_ptr = strings_addr.add(name_off);
                    let mut pname_len = 0;
                    while *name_ptr.add(pname_len) != 0 {
                        pname_len += 1;
                    }
                    let pname_bytes = core::slice::from_raw_parts(name_ptr, pname_len);
                    let pname = core::str::from_utf8(pname_bytes).unwrap_or("?");

                    let value_ptr = pos;
                    let aligned_len = align4(prop_len);
                    pos = pos.add(aligned_len);

                    if inside_gic && pname == "#address-cells" && prop_len >= 4 {
                        if depth < 8 {
                            addr_cells[depth] = read_be_u32(value_ptr);
                        }
                        continue;
                    }
                    if inside_gic && pname == "#size-cells" && prop_len >= 4 {
                        if depth < 8 {
                            size_cells[depth] = read_be_u32(value_ptr);
                        }
                        continue;
                    }

                    if pname == "compatible" && !found_gic {
                        let mut vlen = prop_len;
                        if vlen > 0 && *value_ptr.add(vlen - 1) == 0 {
                            vlen -= 1;
                        }
                        let bytes = core::slice::from_raw_parts(value_ptr, vlen);
                        if let Ok(s) = core::str::from_utf8(bytes) {
                            if s.contains("cortex-a15-gic") || s.contains("gic-v3") {
                                found_gic = true;
                                // Extract the first compatible string
                                let first = bytes.split(|&b| b == 0).next().unwrap_or(b"");
                                compatible = core::str::from_utf8(first).unwrap_or("");
                            }
                        }
                        continue;
                    }

                    if inside_gic && depth == target_depth && pname == "reg" {
                        let parent_ac = if depth > 0 && depth < 8 {
                            addr_cells[depth - 1]
                        } else {
                            2
                        };
                        let parent_sc = if depth > 0 && depth < 8 {
                            size_cells[depth - 1]
                        } else {
                            2
                        };
                        let entry_bytes = ((parent_ac + parent_sc) * 4) as usize;
                        let n_entries = if entry_bytes > 0 {
                            prop_len / entry_bytes
                        } else {
                            0
                        };

                        if n_entries >= 1 {
                            let base = read_be_u64(value_ptr, parent_ac);
                            let size =
                                read_be_u64(value_ptr.add(parent_ac as usize * 4), parent_sc);
                            distributor = Some(MmioRegion { addr: base, size });
                        }
                        if n_entries >= 2 {
                            let off = entry_bytes;
                            let base = read_be_u64(value_ptr.add(off), parent_ac);
                            let size =
                                read_be_u64(value_ptr.add(off + parent_ac as usize * 4), parent_sc);
                            redistributor = Some(MmioRegion { addr: base, size });
                        }
                        continue;
                    }
                }

                FDT_NOP => {}
                FDT_END => break,
                _ => break,
            }
        }

        let dist = distributor?;
        Some(InterruptControllerInfo {
            compatible,
            distributor: dist,
            redistributor,
        })
    }
}
