use crate::state::HardwareComponent;
use crate::dt::types::Fdt;

const STORAGE_COMPATIBLE: &[&str] = &["mmc", "sdhci", "sdmmc", "sdhci-msm", "mmci"];

pub fn discover_from_fdt(fdt: &Fdt) -> Vec<HardwareComponent> {
    let mut components = Vec::new();

    // Discover storage nodes by compatible string
    let storage_nodes = fdt.root.find_nodes_by_compatible(STORAGE_COMPATIBLE);
    for node in storage_nodes {
        let vendor = node.get_property("compatible")
            .and_then(|p| p.as_string())
            .unwrap_or("unknown")
            .to_string();
        let serial = node.get_property("serial-number")
            .and_then(|p| p.as_string())
            .unwrap_or("unknown")
            .to_string();
        let model = node.get_property("model")
            .and_then(|p| p.as_string())
            .unwrap_or(&vendor)
            .to_string();

        components.push(HardwareComponent {
            component_class: "storage".to_string(),
            vendor_id: vendor,
            model_id: model,
            serial_number: serial,
        });
    }

    // If no storage nodes were found via compatible, try memory-mapped MMC
    if components.is_empty() {
        if let Some(mmc_node) = fdt.root.get_child("mmc") {
            let serial = mmc_node.get_property("serial-number")
                .and_then(|p| p.as_string())
                .unwrap_or("unknown")
                .to_string();
            components.push(HardwareComponent {
                component_class: "storage".to_string(),
                vendor_id: "qcom".to_string(),
                model_id: "mmc".to_string(),
                serial_number: serial,
            });
        }
    }

    components
}

pub fn is_storage_replaced(old: &[HardwareComponent], new: &[HardwareComponent]) -> bool {
    if old.is_empty() || new.is_empty() {
        return true;
    }
    old != new
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dt::parser::parse_fdt;

    fn build_test_dtb() -> Vec<u8> {
        let mut dtb = Vec::new();
        fn be32(v: u32) -> [u8; 4] { v.to_be_bytes() }

        let string_entries = [
            "compatible", "model", "serial-number",
        ];
        let mut string_offsets = Vec::new();
        let mut strings_bytes = Vec::new();
        for s in &string_entries {
            string_offsets.push(strings_bytes.len() as u32);
            strings_bytes.extend_from_slice(s.as_bytes());
            strings_bytes.push(0);
        }

        fn emit_prop(dtb: &mut Vec<u8>, name_off: u32, value: &[u8]) {
            let be32 = |v: u32| v.to_be_bytes();
            dtb.extend_from_slice(&be32(0x00000003)); // FDT_PROP
            dtb.extend_from_slice(&be32(value.len() as u32));
            dtb.extend_from_slice(&be32(name_off));
            dtb.extend_from_slice(value);
            while dtb.len() % 4 != 0 { dtb.push(0); }
        }

        dtb.extend_from_slice(&be32(0xD00DFEED));
        dtb.extend_from_slice(&be32(0)); dtb.extend_from_slice(&be32(56));
        dtb.extend_from_slice(&be32(0)); dtb.extend_from_slice(&be32(40));
        dtb.extend_from_slice(&be32(17)); dtb.extend_from_slice(&be32(16));
        dtb.extend_from_slice(&be32(0)); dtb.extend_from_slice(&be32(0));
        dtb.extend_from_slice(&be32(0));
        dtb.extend_from_slice(&be32(0)); dtb.extend_from_slice(&be32(0));
        dtb.extend_from_slice(&be32(0)); dtb.extend_from_slice(&be32(0));

        let struct_start = dtb.len();

        dtb.extend_from_slice(&be32(0x00000001));
        dtb.extend_from_slice(b"/\0"); while dtb.len() % 4 != 0 { dtb.push(0); }

        // sdhci node with storage properties
        dtb.extend_from_slice(&be32(0x00000001));
        dtb.extend_from_slice(b"sdhci@c0c0000\0");
        while dtb.len() % 4 != 0 { dtb.push(0); }

        emit_prop(&mut dtb, string_offsets[0], b"mmc,sdhci-msm\0");
        emit_prop(&mut dtb, string_offsets[1], b"SDM660-eMMC\0");
        emit_prop(&mut dtb, string_offsets[2], b"eMMC-SDM660-001\0");

        dtb.extend_from_slice(&be32(0x00000002));
        dtb.extend_from_slice(&be32(0x00000002));
        dtb.extend_from_slice(&be32(0x00000009));

        let struct_size = dtb.len() - struct_start;
        let strings_start = dtb.len();
        dtb.extend_from_slice(&strings_bytes);
        let strings_size = dtb.len() - strings_start;
        let total_size = dtb.len();

        dtb[4..8].copy_from_slice(&be32(total_size as u32));
        dtb[12..16].copy_from_slice(&be32(strings_start as u32));
        dtb[32..36].copy_from_slice(&be32(strings_size as u32));
        dtb[36..40].copy_from_slice(&be32(struct_size as u32));

        dtb
    }

    #[test]
    fn test_discover_storage_from_fdt() {
        let dtb = build_test_dtb();
        let fdt = parse_fdt(&dtb).unwrap();
        let components = discover_from_fdt(&fdt);

        assert!(!components.is_empty(), "Should discover at least one storage component");
        assert_eq!(components[0].component_class, "storage");
        assert_eq!(components[0].serial_number, "eMMC-SDM660-001");
    }

    #[test]
    fn test_storage_replacement_detected() {
        let old = vec![HardwareComponent {
            component_class: "storage".to_string(),
            vendor_id: "qemu".to_string(),
            model_id: "virtio-blk".to_string(),
            serial_number: "eMMC-0001-QEMU".to_string(),
        }];
        let new = vec![HardwareComponent {
            component_class: "storage".to_string(),
            vendor_id: "qemu".to_string(),
            model_id: "virtio-blk".to_string(),
            serial_number: "eMMC-0002-QEMU".to_string(),
        }];
        assert!(is_storage_replaced(&old, &new));
    }

    #[test]
    fn test_same_storage_not_replaced() {
        let inv = vec![HardwareComponent {
            component_class: "storage".to_string(),
            vendor_id: "qemu".to_string(),
            model_id: "virtio-blk".to_string(),
            serial_number: "eMMC-0001-QEMU".to_string(),
        }];
        assert!(!is_storage_replaced(&inv, &inv));
    }
}
