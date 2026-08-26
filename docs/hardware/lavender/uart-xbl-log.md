# Lavender XBL Boot Log — captured via UART TP11/TP10 (2026-08-23)

Source: `/tmp/lavender-uart-boot.log` (intermittent 0.5mm pad contact)

## XBL (Qualcomm eXtensible Boot Loader)

| Field | Value |
|-------|-------|
| QC_IMAGE_VERSION | BOOT.XF.1.4-00283-S660LZB-4 |
| IMAGE_VARIANT | Sdm660LA |
| OEM_IMAGE_VERSION | c4-miui-ota-bd169.bj |
| Boot Interface | eMMC |
| Secure Boot | **On** |
| JTAG ID | 0x0008c0e1 |
| CPUID | 0x8df62afd |
| Core 0 Frequency | 3715 MHz |
| I-cache / D-cache | On / On |
| PBL Patch Ver | 5 |
| CDT Version | 3, Platform ID: 8, Major: 1, Minor: 0, Subtype: 0 |

## Boot sequence timing (microseconds from PBL start)

| Stage | Ticks | Delta |
|-------|------:|------:|
| PBL start | 0 | — |
| bootable_media_detect_entry | 7027 | — |
| bootable_media_detect_success | 101767 | +94740 |
| elf_loader_entry | 101772 | +5 |
| auth_hash_seg_entry | 103337 | +1565 |
| elf_segs_hash_verify_entry | 182294 | +78957 |
| elf_segs_hash_verify_exit | 232536 | +50242 |
| xbl_sec auth complete | 268324 | +35788 |
| **PBL End** | **268369** | — |
| SBL1 start | 296216 | +27847 |
| USB HS PHY nondrive | 412055 | +115839 |
| boot_flash_init | 415288 | +3233 |
| DDR default params | 422669 | +7381 |
| CDT Version:3 Platform:8 | 451857 | +29188 |
| Segments hash check | (cut off) | — |

## Notes

- UART: BLSP1_UART2 @ 0x0C170000, 115200 8N1, TP11=TX(gpio4), TP10=RX(gpio5)
- XBL boots: eMMC→elf_loader→hash_verify→SBL1→DDR→CDT→(ABL not captured due to contact drops)
- Phone rebooted twice during capture (two XBL cycles in log)
- ABL phase (where unlock state lives) was never captured — UART drops at DDR/CDT stage
- Full ABL + kernel log requires stable UART contact (clamp/tape recommended)
