use core::fmt;

#[derive(Debug, Clone)]
pub struct FdtHeader {
    pub magic: u32,
    pub totalsize: u32,
    pub off_dt_struct: u32,
    pub off_dt_strings: u32,
    pub off_mem_rsvmap: u32,
    pub version: u32,
    pub last_comp_version: u32,
    pub boot_cpuid_phys: u32,
    pub size_dt_strings: u32,
    pub size_dt_struct: u32,
}

impl FdtHeader {
    pub fn validate(&self) -> bool {
        self.magic == 0xD00DFEED
            && self.version >= 16
            && self.last_comp_version <= 16
            && self.totalsize > 0
    }
}

#[derive(Debug, Clone)]
pub struct FdtProperty {
    pub name: String,
    pub value: Vec<u8>,
}

impl FdtProperty {
    pub fn as_string(&self) -> Option<&str> {
        let end = self.value.iter().position(|&b| b == 0).unwrap_or(self.value.len());
        core::str::from_utf8(&self.value[..end]).ok()
    }

    pub fn as_u32(&self) -> Option<u32> {
        if self.value.len() < 4 { return None; }
        Some(u32::from_be_bytes([self.value[0], self.value[1], self.value[2], self.value[3]]))
    }

    pub fn as_u64(&self) -> Option<u64> {
        if self.value.len() < 8 { return None; }
        Some(u64::from_be_bytes([
            self.value[0], self.value[1], self.value[2], self.value[3],
            self.value[4], self.value[5], self.value[6], self.value[7],
        ]))
    }
}

#[derive(Debug, Clone)]
pub struct FdtNode {
    pub name: String,
    pub properties: Vec<FdtProperty>,
    pub children: Vec<FdtNode>,
}

impl FdtNode {
    pub fn get_property(&self, name: &str) -> Option<&FdtProperty> {
        self.properties.iter().find(|p| p.name == name)
    }

    pub fn get_child(&self, name: &str) -> Option<&FdtNode> {
        self.children.iter().find(|c| c.name == name || c.name.trim_end_matches('/') == name)
    }

    pub fn find_nodes_by_compatible(&self, compat: &[&str]) -> Vec<&FdtNode> {
        let mut result = Vec::new();
        if let Some(prop) = self.get_property("compatible") {
            if let Some(val) = prop.as_string() {
                if compat.iter().any(|c| val.contains(c)) {
                    result.push(self);
                }
            }
        }
        for child in &self.children {
            result.extend(child.find_nodes_by_compatible(compat));
        }
        result
    }
}

impl fmt::Display for FdtNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_tree(f, 0)
    }
}

impl FdtNode {
    fn fmt_tree(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        let indent = "  ".repeat(depth);
        writeln!(f, "{} {} {{", indent, self.name)?;
        for prop in &self.properties {
            let val = prop.as_string().unwrap_or("<binary>");
            writeln!(f, "{}  {} = \"{}\";", indent, prop.name, val)?;
        }
        for child in &self.children {
            child.fmt_tree(f, depth + 1)?;
        }
        writeln!(f, "{}}};", indent)
    }
}

pub struct Fdt {
    pub header: FdtHeader,
    pub root: FdtNode,
}

impl Fdt {
    pub fn get_model(&self) -> Option<&str> {
        self.root.get_property("model").and_then(|p| p.as_string())
    }

    pub fn get_compatible(&self) -> Option<&str> {
        self.root.get_property("compatible").and_then(|p| p.as_string())
    }
}

impl fmt::Display for Fdt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "/dts-v1/;")?;
        writeln!(f, "// magic=0x{:08X}, size={}", self.header.magic, self.header.totalsize)?;
        write!(f, "{}", self.root)
    }
}

pub enum DtbError {
    InvalidMagic(u32),
    InvalidVersion(u32, u32),
    StructureParseFailed,
    StringBlockCorrupt,
    EndTokenMissing,
}

impl fmt::Debug for DtbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DtbError::InvalidMagic(m) => write!(f, "Invalid DTB magic: 0x{:08X} (expected 0xD00DFEED)", m),
            DtbError::InvalidVersion(v, l) => write!(f, "Unsupported DTB version: {} (last compat: {})", v, l),
            DtbError::StructureParseFailed => write!(f, "Failed to parse DTB structure block"),
            DtbError::StringBlockCorrupt => write!(f, "DTB string block corrupt"),
            DtbError::EndTokenMissing => write!(f, "DTB structure block missing END token"),
        }
    }
}
