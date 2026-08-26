extern crate std;
use std::vec;
extern crate std;
use crate::elf::*;
use crate::elf::{ElfError, LoadPlan, plan_load};
use std::vec::Vec;

const PHENTSIZE: usize = 56;

struct Seg {
    vaddr: u64,
    offset: u32,
    filesz: u32,
    memsz: u32,
    flags: u32,
}

struct Builder {
    entry: u64,
    segs: Vec<Seg>,
    file_len: usize,
}

impl Builder {
    fn new(entry: u64) -> Self {
        Builder {
            entry,
            segs: Vec::new(),
            file_len: 64 + PHENTSIZE,
        }
    }
    /// Places file data at an offset congruent to `vaddr` modulo PAGE
    /// (`offset ≡ vaddr (mod PAGE)`), matching the ELF System V ABI
    /// invariant that our loader validates.
    fn seg_at(mut self, offset_page: u32, vaddr: u64, filesz: u32, memsz: u32, flags: u32) -> Self {
        let off = (offset_page * 0x1000) + (vaddr % crate::elf::PAGE) as u32;
        self.segs.push(Seg {
            vaddr,
            offset: off,
            filesz,
            memsz,
            flags,
        });
        let end = off as usize + filesz as usize;
        if end > self.file_len {
            self.file_len = end;
        }
        self
    }
    fn build(&self) -> Vec<u8> {
        let phoff = 64u64;
        let total = (phoff as usize) + PHENTSIZE * self.segs.len().max(1);
        let mut img = vec![0u8; total.max(self.file_len)];
        img[0..4].copy_from_slice(b"\x7fELF");
        img[4] = 2;
        img[5] = 1;
        img[16..18].copy_from_slice(&2u16.to_le_bytes());
        img[18..20].copy_from_slice(&0xB7u16.to_le_bytes());
        img[24..32].copy_from_slice(&self.entry.to_le_bytes());
        img[32..40].copy_from_slice(&phoff.to_le_bytes());
        img[54..56].copy_from_slice(&(PHENTSIZE as u16).to_le_bytes());
        img[56..58].copy_from_slice(&(self.segs.len() as u16).to_le_bytes());
        for (i, s) in self.segs.iter().enumerate() {
            let base = phoff as usize + i * PHENTSIZE;
            img[base..base + 4].copy_from_slice(&1u32.to_le_bytes());
            img[base + 4..base + 8].copy_from_slice(&s.flags.to_le_bytes());
            img[base + 8..base + 16].copy_from_slice(&(s.offset as u64).to_le_bytes());
            img[base + 16..base + 24].copy_from_slice(&s.vaddr.to_le_bytes());
            img[base + 32..base + 40].copy_from_slice(&(s.filesz as u64).to_le_bytes());
            img[base + 40..base + 48].copy_from_slice(&(s.memsz as u64).to_le_bytes());
        }
        img
    }
}

fn good_image() -> Vec<u8> {
    Builder::new(0x0040_0008)
        .seg_at(1, 0x0040_0000, 0x100, 0x100, PF_R | PF_X)
        .seg_at(2, 0x0040_1000, 0x80, 0x1080, PF_R | PF_W)
        .build()
}

#[test]
fn accepts_valid_static_image() {
    let plan = plan_load(&good_image()).expect("valid image must plan");
    assert_eq!(plan.entry, 0x0040_0008);
    assert_eq!(plan.segments.len(), 2);
    let mut it = (&plan.segments).into_iter();
    let text = it.next().unwrap();
    assert_eq!(text.va_start, 0x0040_0000);
    assert_eq!(text.filesz, 0x100);
    assert!(text.flags & PF_X != 0);
    let data = it.next().unwrap();
    assert_eq!(data.va_start, 0x0040_1000);
    // memsz end = 0x401000 + 0x1080 = 0x402080 → page-rounds to 0x403000.
    assert_eq!(data.va_end, 0x0040_3000);
    // filesz ends at 0x401080 → page-rounds to 0x402000; lazy tail is
    // [0x402000, 0x403000) — one full BSS page.
    assert_eq!(data.lazy_tail(), Some((0x0040_2000, 0x1000)));
}

#[test]
fn lazy_tail_math() {
    // Entry sits inside the executable segment so the plan is accepted.
    let img = Builder::new(0x0040_1008)
        .seg_at(2, 0x0040_1000, 0x10, 0x2080, PF_R | PF_X)
        .build();
    let plan = plan_load(&img).expect("lazy_tail_math: image must plan");
    let s = (&plan.segments).into_iter().next().expect("one segment");
    // filesz ends at 0x401010 → page-rounds to 0x402000; memsz end 0x403080
    // rounds to 0x404000. Lazy tail = [0x402000, 0x404000).
    assert_eq!(s.lazy_tail(), Some((0x0040_2000, 0x2000)));
}

#[test]
fn rejects_bad_magic_and_class() {
    let mut img = good_image();
    img[0] = 0x7E;
    assert_eq!(plan_load(&img), Err(ElfError::BadMagic));
    let mut img = good_image();
    img[4] = 1;
    assert_eq!(plan_load(&img), Err(ElfError::BadClass));
}

#[test]
fn rejects_wrong_machine_and_type() {
    let mut img = good_image();
    img[18..20].copy_from_slice(&3u16.to_le_bytes());
    assert_eq!(plan_load(&img), Err(ElfError::BadMachine));
    let mut img = good_image();
    img[16..18].copy_from_slice(&3u16.to_le_bytes());
    assert_eq!(plan_load(&img), Err(ElfError::BadType));
}

#[test]
fn rejects_truncated_header_table() {
    let mut img = Builder::new(0x0040_0008)
        .seg_at(1, 0x0040_0000, 0x10, 0x10, PF_R | PF_X)
        .build();
    plan_load(&img).expect("precondition: compact image must be valid");
    img[56..58].copy_from_slice(&255u16.to_le_bytes());
    assert_eq!(plan_load(&img), Err(ElfError::TruncatedHeader));
}

#[test]
fn rejects_segment_file_truncation() {
    let mut img = Builder::new(0x0040_0008)
        .seg_at(1, 0x0040_0000, 0xFFFF, 0xFFFF, PF_R | PF_X)
        .build();
    img.truncate(0x110);
    assert_eq!(plan_load(&img), Err(ElfError::SegmentTruncated));
}

#[test]
fn rejects_overlapping_segments() {
    let img = Builder::new(0x0040_0008)
        .seg_at(1, 0x0040_0000, 0x3000, 0x3000, PF_R | PF_X)
        .seg_at(9, 0x0040_1000, 0x10, 0x10, PF_R | PF_W)
        .build();
    assert_eq!(plan_load(&img), Err(ElfError::SegmentOverlap));
}

#[test]
fn rejects_wx_segment() {
    let img = Builder::new(0x0040_0008)
        .seg_at(1, 0x0040_0000, 0x100, 0x100, PF_R | PF_W | PF_X)
        .build();
    assert!(plan_load(&img).is_err());
}

#[test]
fn rejects_entry_outside_exec() {
    let img = Builder::new(0x0040_1008)
        .seg_at(1, 0x0040_0000, 0x100, 0x100, PF_R | PF_X)
        .seg_at(2, 0x0040_1000, 0x80, 0x80, PF_R | PF_W)
        .build();
    assert_eq!(plan_load(&img), Err(ElfError::BadEntry));
}

#[test]
fn rejects_memsz_below_filesz() {
    let img = Builder::new(0x0040_0008)
        .seg_at(1, 0x0040_0000, 0x200, 0x100, PF_R | PF_X)
        .build();
    assert_eq!(plan_load(&img), Err(ElfError::BadSegmentSize));
}
