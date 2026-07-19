use vivanta_boot_common::{MemoryMap, MemoryRegion, MemoryRegionKind, println};

const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32   = 0x00000002;
const FDT_PROP: u32       = 0x00000003;
const FDT_NOP: u32        = 0x00000004;
const FDT_END: u32        = 0x00000009;

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

pub struct FdtScanner;

impl FdtScanner {
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

        println!("FDT: magic OK, version {}, total {} bytes", version, totalsize);

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
            if pos.add(4) > struct_end { break; }
            let token = read_be_u32(pos);
            pos = pos.add(4);

            match token {
                FDT_BEGIN_NODE => {
                    let name_start = pos;
                    let mut name_len = 0;
                    while *name_start.add(name_len) != 0 { name_len += 1; }
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

                FDT_END_NODE => { depth -= 1; }

                FDT_PROP => {
                    if pos.add(8) > struct_end { break; }
                    let prop_len = read_be_u32(pos) as usize;
                    let name_off = read_be_u32(pos.add(4)) as usize;
                    pos = pos.add(8);

                    let name_ptr = strings_addr.add(name_off);
                    let mut pname_len = 0;
                    while *name_ptr.add(pname_len) != 0 { pname_len += 1; }
                    let pname_bytes = core::slice::from_raw_parts(name_ptr, pname_len);
                    let pname = core::str::from_utf8(pname_bytes).unwrap_or("?");
                    let value_ptr = pos;
                    pos = pos.add(align4(prop_len));

                    if pname == "#address-cells" && prop_len >= 4 {
                        let v = read_be_u32(value_ptr);
                        if depth < 8 { addr_cells[depth] = v; }
                        continue;
                    }
                    if pname == "#size-cells" && prop_len >= 4 {
                        let v = read_be_u32(value_ptr);
                        if depth < 8 { size_cells[depth] = v; }
                        continue;
                    }

                    if pname == "reg" && current_name.starts_with("memory") {
                        let parent_ac = if depth > 1 && depth - 1 < 8 { addr_cells[depth - 1] } else { 2 };
                        let parent_sc = if depth > 1 && depth - 1 < 8 { size_cells[depth - 1] } else { 2 };
                        let entry_bytes = ((parent_ac + parent_sc) * 4) as usize;
                        let n_entries = if entry_bytes > 0 { prop_len / entry_bytes } else { 0 };
                        for i in 0..n_entries {
                            let off = i * entry_bytes;
                            let base = read_be_u64(value_ptr.add(off), parent_ac);
                            let size = read_be_u64(value_ptr.add(off + parent_ac as usize * 4), parent_sc);
                            mem.push(MemoryRegion { start: base, size, kind: MemoryRegionKind::Usable });
                            println!("  FDT:   RAM 0x{:016x} – 0x{:016x}  ({} MiB)",
                                base, base + size - 1, size >> 20);
                        }
                        continue;
                    }

                    let mut vlen = prop_len;
                    if vlen > 0 && *value_ptr.add(vlen - 1) == 0 { vlen -= 1; }
                    let value_bytes = core::slice::from_raw_parts(value_ptr, vlen);
                    let value_str = core::str::from_utf8(value_bytes).unwrap_or("<binary>");
                    let indent = if depth <= 1 { "" } else { "  " };
                    match pname {
                        "model" | "compatible" => println!("  FDT: {} {} = {}", indent, current_name, value_str),
                        "device_type" if value_str == "memory" => println!("  FDT: {} memory node found", indent),
                        "stdout-path" => println!("  FDT: {} console: {}", indent, value_str),
                        _ => {}
                    }
                }

                FDT_NOP => {}
                FDT_END => break,
                _ => { println!("FDT: unknown token 0x{:08X}", token); break; }
            }
        }

        println!("FDT: scan complete – {} memory region(s)", mem.regions().len());
        true
    }
}
