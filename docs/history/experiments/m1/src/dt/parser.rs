use super::types::*;

const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32   = 0x00000002;
const FDT_PROP: u32       = 0x00000003;
const FDT_NOP: u32        = 0x00000004;
const FDT_END: u32        = 0x00000009;

fn read_be_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
    ])
}

fn parse_header(data: &[u8]) -> Result<FdtHeader, DtbError> {
    if data.len() < 40 {
        return Err(DtbError::StructureParseFailed);
    }

    let header = FdtHeader {
        magic: read_be_u32(data, 0),
        totalsize: read_be_u32(data, 4),
        off_dt_struct: read_be_u32(data, 8),
        off_dt_strings: read_be_u32(data, 12),
        off_mem_rsvmap: read_be_u32(data, 16),
        version: read_be_u32(data, 20),
        last_comp_version: read_be_u32(data, 24),
        boot_cpuid_phys: read_be_u32(data, 28),
        size_dt_strings: read_be_u32(data, 32),
        size_dt_struct: read_be_u32(data, 36),
    };

    if !header.validate() {
        return Err(DtbError::InvalidMagic(header.magic));
    }
    if header.version < 16 || header.last_comp_version > 16 {
        return Err(DtbError::InvalidVersion(header.version, header.last_comp_version));
    }

    Ok(header)
}

fn get_string(strings_block: &[u8], offset: u32) -> Result<String, DtbError> {
    let off = offset as usize;
    if off >= strings_block.len() {
        return Err(DtbError::StringBlockCorrupt);
    }
    let end = strings_block[off..].iter().position(|&b| b == 0).unwrap_or(strings_block.len() - off);
    Ok(String::from_utf8_lossy(&strings_block[off..off + end]).to_string())
}

fn align_up(offset: usize, align: usize) -> usize {
    (offset + align - 1) & !(align - 1)
}

/// Parse a single node starting at `pos`, updating `pos` to point past the node.
/// Returns the parsed node.
fn parse_node(data: &[u8], pos: &mut usize, end: usize, strings_block: &[u8]) -> Result<FdtNode, DtbError> {
    if *pos + 4 > end {
        return Err(DtbError::StructureParseFailed);
    }

    let token = read_be_u32(data, *pos);
    if token != FDT_BEGIN_NODE {
        return Err(DtbError::StructureParseFailed);
    }
    *pos += 4;

    // Read node name (null-terminated)
    let name_start = *pos;
    while *pos < end && data[*pos] != 0 {
        *pos += 1;
    }
    if *pos >= end {
        return Err(DtbError::StructureParseFailed);
    }
    let node_name = String::from_utf8_lossy(&data[name_start..*pos]).to_string();
    *pos += 1; // skip null
    *pos = align_up(*pos, 4);

    let mut node = FdtNode {
        name: node_name,
        properties: Vec::new(),
        children: Vec::new(),
    };

    loop {
        if *pos + 4 > end {
            return Err(DtbError::EndTokenMissing);
        }

        let token = read_be_u32(data, *pos);
        *pos += 4;

        match token {
            FDT_PROP => {
                if *pos + 8 > end {
                    return Err(DtbError::StructureParseFailed);
                }
                let prop_len = read_be_u32(data, *pos) as usize;
                let name_off = read_be_u32(data, *pos + 4);
                *pos += 8;

                let prop_name = get_string(strings_block, name_off)?;

                if *pos + prop_len > end {
                    return Err(DtbError::StructureParseFailed);
                }
                let prop_value = data[*pos..*pos + prop_len].to_vec();
                *pos += prop_len;
                *pos = align_up(*pos, 4);

                node.properties.push(FdtProperty {
                    name: prop_name,
                    value: prop_value,
                });
            }
            FDT_BEGIN_NODE => {
                *pos -= 4; // back up so parse_node sees the BEGIN_NODE token
                let child = parse_node(data, pos, end, strings_block)?;
                node.children.push(child);
            }
            FDT_END_NODE => {
                return Ok(node);
            }
            FDT_NOP => {}
            FDT_END => {
                return Ok(node);
            }
            _ => {
                return Err(DtbError::StructureParseFailed);
            }
        }
    }
}

pub fn parse_fdt(data: &[u8]) -> Result<Fdt, DtbError> {
    let header = parse_header(data)?;

    let struct_start = header.off_dt_struct as usize;
    let struct_size = header.size_dt_struct as usize;
    let strings_start = header.off_dt_strings as usize;
    let strings_size = header.size_dt_strings as usize;

    if struct_start + struct_size > data.len() || strings_start + strings_size > data.len() {
        return Err(DtbError::StructureParseFailed);
    }

    let strings_block = &data[strings_start..strings_start + strings_size];
    let mut pos = struct_start;
    let struct_end = struct_start + struct_size;
    let root = parse_node(data, &mut pos, struct_end, strings_block)?;

    Ok(Fdt { header, root })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emit_prop(dtb: &mut Vec<u8>, name_off: u32, value: &[u8]) {
        let be32 = |v: u32| v.to_be_bytes();
        dtb.extend_from_slice(&be32(FDT_PROP));
        dtb.extend_from_slice(&be32(value.len() as u32));
        dtb.extend_from_slice(&be32(name_off));
        dtb.extend_from_slice(value);
        while dtb.len() % 4 != 0 { dtb.push(0); }
    }

    fn build_minimal_dtb() -> Vec<u8> {
        let mut dtb = Vec::new();
        fn be32(v: u32) -> [u8; 4] { v.to_be_bytes() }

        // Step 1: collect all strings and compute offsets
        let string_entries = [
            "model", "compatible", "mmc,sdhci", "serial-number", "reg", "device_type", "memory",
        ];
        let mut string_offsets: Vec<u32> = Vec::new();
        let mut strings_bytes = Vec::new();
        for s in &string_entries {
            string_offsets.push(strings_bytes.len() as u32);
            strings_bytes.extend_from_slice(s.as_bytes());
            strings_bytes.push(0);
        }

        // Header
        dtb.extend_from_slice(&be32(0xD00DFEED));
        dtb.extend_from_slice(&be32(0));          // totalsize (patch)
        dtb.extend_from_slice(&be32(56));          // off_dt_struct = after header + mem reserve
        dtb.extend_from_slice(&be32(0));           // off_dt_strings (patch)
        dtb.extend_from_slice(&be32(40));          // off_mem_rsvmap = after 40-byte header
        dtb.extend_from_slice(&be32(17));          // version
        dtb.extend_from_slice(&be32(16));          // last_comp_version
        dtb.extend_from_slice(&be32(0));           // boot_cpuid_phys
        dtb.extend_from_slice(&be32(0));           // size_dt_strings (patch)
        dtb.extend_from_slice(&be32(0));           // size_dt_struct (patch)

        // Memory reservation block
        dtb.extend_from_slice(&be32(0));
        dtb.extend_from_slice(&be32(0));
        dtb.extend_from_slice(&be32(0));
        dtb.extend_from_slice(&be32(0));

        let struct_start = dtb.len();

        // Root node
        dtb.extend_from_slice(&be32(FDT_BEGIN_NODE));
        dtb.extend_from_slice(b"/\0");
        while dtb.len() % 4 != 0 { dtb.push(0); }

        emit_prop(&mut dtb, string_offsets[0], b"lavender\0");
        emit_prop(&mut dtb, string_offsets[1], b"xiaomi,sdm660\0");

        // mmc@0 node
        dtb.extend_from_slice(&be32(FDT_BEGIN_NODE));
        dtb.extend_from_slice(b"mmc@0\0");
        while dtb.len() % 4 != 0 { dtb.push(0); }

        emit_prop(&mut dtb, string_offsets[1], b"mmc,sdhci\0");
        emit_prop(&mut dtb, string_offsets[3], b"eMMC-ABC123\0");
        emit_prop(&mut dtb, string_offsets[4], &[0u8; 8]);

        dtb.extend_from_slice(&be32(FDT_END_NODE));

        // memory@0 node
        dtb.extend_from_slice(&be32(FDT_BEGIN_NODE));
        dtb.extend_from_slice(b"memory@0\0");
        while dtb.len() % 4 != 0 { dtb.push(0); }

        emit_prop(&mut dtb, string_offsets[5], b"memory\0");
        let mem_reg: [u8; 16] = [
            0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
        ];
        emit_prop(&mut dtb, string_offsets[4], &mem_reg);

        dtb.extend_from_slice(&be32(FDT_END_NODE));
        dtb.extend_from_slice(&be32(FDT_END_NODE));
        dtb.extend_from_slice(&be32(FDT_END));

        let struct_end = dtb.len();
        let struct_size = struct_end - struct_start;

        // Strings block
        let strings_start = align_up(struct_end, 4);
        dtb.resize(strings_start, 0);
        dtb.extend_from_slice(&strings_bytes);

        let strings_size = dtb.len() - strings_start;
        let total_size = dtb.len();

        // Patch header
        dtb[4..8].copy_from_slice(&be32(total_size as u32));
        dtb[12..16].copy_from_slice(&be32(strings_start as u32));
        dtb[32..36].copy_from_slice(&be32(strings_size as u32));
        dtb[36..40].copy_from_slice(&be32(struct_size as u32));

        dtb
    }

    #[test]
    fn test_fdt_header_parse() {
        let dtb = build_minimal_dtb();
        let fdt = parse_fdt(&dtb).unwrap();
        assert_eq!(fdt.header.magic, 0xD00DFEED);
        assert_eq!(fdt.header.version, 17);
    }

    #[test]
    fn test_fdt_model_extraction() {
        let dtb = build_minimal_dtb();
        let fdt = parse_fdt(&dtb).unwrap();
        assert_eq!(fdt.get_model(), Some("lavender"));
    }

    #[test]
    fn test_fdt_compatible_extraction() {
        let dtb = build_minimal_dtb();
        let fdt = parse_fdt(&dtb).unwrap();
        assert_eq!(fdt.get_compatible(), Some("xiaomi,sdm660"));
    }

    #[test]
    fn test_fdt_invalid_magic() {
        let mut dtb = build_minimal_dtb();
        dtb[0] = 0xDE;
        dtb[1] = 0xAD;
        assert!(parse_fdt(&dtb).is_err());
    }

    #[test]
    fn test_fdt_storage_node_found() {
        let dtb = build_minimal_dtb();
        let fdt = parse_fdt(&dtb).unwrap();
        let nodes = fdt.root.find_nodes_by_compatible(&["mmc", "sdhci"]);
        assert!(!nodes.is_empty(), "Should find at least one storage node; tree:\n{}", fdt);
        if let Some(serial) = nodes[0].get_property("serial-number") {
            assert_eq!(serial.as_string(), Some("eMMC-ABC123"));
        } else {
            panic!("serial-number property not found on storage node");
        }
    }

    #[test]
    fn test_fdt_strings_integrity() {
        let dtb = build_minimal_dtb();
        // Corrupt the strings block offset
        let mut corrupted = dtb.clone();
        let strings_off_pos = 12;
        corrupted[strings_off_pos..strings_off_pos + 4].copy_from_slice(&9999u32.to_be_bytes());
        assert!(parse_fdt(&corrupted).is_err());
    }
}
