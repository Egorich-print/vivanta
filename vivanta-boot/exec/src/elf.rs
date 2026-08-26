//! Minimal ELF64 little-endian AArch64 program-image reader.
//!
//! Supported: `ET_EXEC` images with `PT_LOAD` program headers. Everything
//! else (dynamic linking, relocations, non-AArch64) is rejected at plan
//! time — a static bare-metal-style userland is the M8 contract.

/// File offset / memory address type in the image.
pub type Off = u64;
pub type Addr = u64;

pub const ELF_MAGIC: [u8; 4] = *b"\x7fELF";
pub const ELFCLASS64: u8 = 2;
pub const ELFDATA2LSB: u8 = 1;
pub const EM_AARCH64: u16 = 0xB7;
pub const ET_EXEC: u16 = 2;

pub const PT_LOAD: u32 = 1;

pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

/// Page size of the target VA space (Vivanta: 4 KiB).
pub const PAGE: u64 = 4096;
/// Entry-point alignment requirement (instruction fetch alignment).
pub const ENTRY_ALIGN: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    TooSmall,
    BadMagic,
    BadClass,
    BadEndian,
    BadMachine,
    BadType,
    BadHeaderFields,  // e_phoff==0 / e_phnum==0 / e_phentsize too small
    TruncatedHeader,  // program header table extends past image
    SegmentTruncated, // p_filesz extends past image
    SegmentOverlap,   // two PT_LOADs overlap after page-rounding
    BadSegmentSize,   // p_memsz < p_filesz
    NoLoadSegments,   // nothing to load
    BadEntry,         // entry not inside any executable segment
    BadAlignment,     // segment vaddr/offset congruence violated
    Overflow,
}

fn rd16(img: &[u8], off: usize) -> Option<u16> {
    img.get(off..off + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}
fn rd32(img: &[u8], off: usize) -> Option<u32> {
    img.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn rd64(img: &[u8], off: usize) -> Option<u64> {
    img.get(off..off + 8)
        .map(|b| u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}

/// One validated PT_LOAD segment, page-rounded for mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadSegment {
    /// Page-aligned virtual start.
    pub va_start: Addr,
    /// Exclusive virtual end (page-rounded up from vaddr+memsz).
    pub va_end: Addr,
    /// Byte offset of file data inside the image.
    pub file_off: Off,
    /// Bytes to copy from the file (filesz).
    pub filesz: u64,
    /// Total bytes in memory (memsz ≥ filesz; tail is zero-fill/lazy).
    pub memsz: u64,
    /// Decoded permissions (R/W/X bits, same values as PF_*).
    pub flags: u32,
}

impl LoadSegment {
    /// True when the segment's file tail must be copied eagerly.
    pub fn has_file_data(&self) -> bool {
        self.filesz > 0
    }

    /// Lazy (demand-fillable) tail: memsz beyond filesz, page-rounded.
    pub fn lazy_tail(&self) -> Option<(Addr, u64)> {
        // Round filesz end up to page boundary — partial last data page is
        // copied then zero-extended by the loader.
        let data_end = self.va_start.saturating_add(self.filesz);
        let data_end_r = (data_end + PAGE - 1) & !(PAGE - 1);
        if self.va_end > data_end_r {
            Some((data_end_r, self.va_end - data_end_r))
        } else {
            None
        }
    }
}

/// Complete validated load plan for an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadPlan {
    pub entry: Addr,
    pub segments: alloc_vec::Vec<LoadSegment>,
}

/// Minimal fixed-capacity Vec stand-in so the crate can be `no_std`
/// without an allocator dependency while keeping the API ergonomic.
mod alloc_vec {
    use core::slice;
    pub struct Vec<T> {
        buf: [Option<T>; MAX_SEGS],
        len: usize,
    }
    pub const MAX_SEGS: usize = 16;

    impl<T: Copy> Vec<T> {
        pub fn new() -> Self {
            Vec {
                buf: [None; MAX_SEGS],
                len: 0,
            }
        }
        pub fn push(&mut self, v: T) -> Result<(), ()> {
            if self.len >= MAX_SEGS {
                return Err(());
            }
            self.buf[self.len] = Some(v);
            self.len += 1;
            Ok(())
        }
        pub fn iter(&self) -> slice::Iter<'_, Option<T>> {
            self.buf[..self.len].iter()
        }
        pub fn len(&self) -> usize {
            self.len
        }
    }

    impl<T: Copy + core::fmt::Debug> Vec<T> {
        pub fn as_slice(&self) -> &[Option<T>] {
            &self.buf[..self.len]
        }
    }
    impl<T: Copy + core::fmt::Debug> core::fmt::Debug for Vec<T> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_list()
                .entries(self.buf[..self.len].iter().map(|o| o.as_ref().unwrap()))
                .finish()
        }
    }
    impl<T: Copy + PartialEq> PartialEq for Vec<T> {
        fn eq(&self, other: &Self) -> bool {
            self.len == other.len
                && self.buf[..self.len]
                    .iter()
                    .zip(other.buf[..other.len].iter())
                    .all(|(a, b)| match (a, b) {
                        (Some(x), Some(y)) => x == y,
                        _ => false,
                    })
        }
    }
    impl<T: Copy + Eq> Eq for Vec<T> {}
    impl<T: Copy> Clone for Vec<T> {
        fn clone(&self) -> Self {
            let mut c = Vec {
                buf: [None; MAX_SEGS],
                len: self.len,
            };
            for i in 0..self.len {
                c.buf[i] = self.buf[i];
            }
            c
        }
    }

    impl<'a, T: Copy> IntoIterator for &'a Vec<T> {
        type Item = T;
        type IntoIter = FilterMapI<'a, T>;
        fn into_iter(self) -> Self::IntoIter {
            FilterMapI {
                inner: self.iter(),
                _pd: core::marker::PhantomData,
            }
        }
    }
    pub struct FilterMapI<'a, T> {
        inner: slice::Iter<'a, Option<T>>,
        _pd: core::marker::PhantomData<T>,
    }
    impl<'a, T: Copy> Iterator for FilterMapI<'a, T> {
        type Item = T;
        fn next(&mut self) -> Option<T> {
            self.inner.next().and_then(|o| *o)
        }
    }
}

/// Parse and validate the image into a [`LoadPlan`].
pub fn plan_load(img: &[u8]) -> Result<LoadPlan, ElfError> {
    // ---- ELF header (52..64 bytes fixed layout for ELF64) ---------------
    if img.len() < 64 {
        return Err(ElfError::TooSmall);
    }
    if img[0..4] != ELF_MAGIC {
        return Err(ElfError::BadMagic);
    }
    if img[4] != ELFCLASS64 {
        return Err(ElfError::BadClass);
    }
    if img[5] != ELFDATA2LSB {
        return Err(ElfError::BadEndian);
    }
    let e_type = rd16(img, 16).ok_or(ElfError::TooSmall)?;
    let e_machine = rd16(img, 18).ok_or(ElfError::TooSmall)?;
    let e_entry = rd64(img, 24).ok_or(ElfError::TooSmall)?;
    let e_phoff = rd64(img, 32).ok_or(ElfError::TooSmall)?;
    let e_phentsize = rd16(img, 54).ok_or(ElfError::TooSmall)?;
    let e_phnum = rd16(img, 56).ok_or(ElfError::TooSmall)?;

    if e_type != ET_EXEC {
        return Err(ElfError::BadType);
    }
    if e_machine != EM_AARCH64 {
        return Err(ElfError::BadMachine);
    }
    // Program header table must exist and entries must be big enough for
    // every field we read (through p_memsz at offset 40..48).
    if e_phoff == 0 || e_phnum == 0 || (e_phentsize as usize) < 56 {
        return Err(ElfError::BadHeaderFields);
    }
    let table_bytes = (e_phoff)
        .checked_add((e_phnum as u64) * (e_phentsize as u64))
        .ok_or(ElfError::Overflow)?;
    if table_bytes as usize > img.len() {
        return Err(ElfError::TruncatedHeader);
    }

    // ---- Program headers -------------------------------------------------
    let mut segments = alloc_vec::Vec::new();
    for i in 0..e_phnum as usize {
        let base = e_phoff as usize + i * e_phentsize as usize;
        let p_type = rd32(img, base + 0).ok_or(ElfError::TruncatedHeader)?;
        let p_flags = rd32(img, base + 4).ok_or(ElfError::TruncatedHeader)?;
        let p_offset = rd64(img, base + 8).ok_or(ElfError::TruncatedHeader)?;
        let p_vaddr = rd64(img, base + 16).ok_or(ElfError::TruncatedHeader)?;
        let p_filesz = rd64(img, base + 32).ok_or(ElfError::TruncatedHeader)?;
        let p_memsz = rd64(img, base + 40).ok_or(ElfError::TruncatedHeader)?;

        if p_type != PT_LOAD {
            continue;
        }
        // Congruence: (vaddr - offset) mod page == 0 is required for a
        // single-mapping loader.
        if (p_vaddr ^ p_offset) & (PAGE - 1) != 0 {
            return Err(ElfError::BadAlignment);
        }
        if p_memsz < p_filesz {
            return Err(ElfError::BadSegmentSize);
        }
        // File slice bounds-checked against the whole image.
        let file_end = p_offset.checked_add(p_filesz).ok_or(ElfError::Overflow)?;
        if (file_end as usize) > img.len() {
            return Err(ElfError::SegmentTruncated);
        }
        // Memory extent overflow-checked and page-rounded.
        let mem_end = p_vaddr.checked_add(p_memsz).ok_or(ElfError::Overflow)?;
        let va_start = p_vaddr & !(PAGE - 1);
        let va_end = (mem_end + PAGE - 1) & !(PAGE - 1);
        if va_end <= va_start && p_memsz > 0 {
            return Err(ElfError::Overflow);
        }
        // W^X policy: writable AND executable segments are rejected here,
        // before the kernel sees the plan.
        if p_flags & PF_W != 0 && p_flags & PF_X != 0 {
            return Err(ElfError::BadSegmentSize); // W^X violation surfaced as plan rejection
        }

        segments
            .push(LoadSegment {
                va_start,
                va_end,
                file_off: p_offset,
                filesz: p_filesz,
                memsz: p_memsz,
                flags: p_flags,
            })
            .map_err(|_| ElfError::NoLoadSegments)?;
    }

    // ---- Cross-segment checks --------------------------------------------
    if segments.len() == 0 {
        return Err(ElfError::NoLoadSegments);
    }
    // Disjoint after page-rounding (sorted copy for overlap detection).
    let mut sorted: [Option<(Addr, Addr)>; alloc_vec::MAX_SEGS] = [None; alloc_vec::MAX_SEGS];
    let mut n = 0usize;
    for s in &segments {
        sorted[n] = Some((s.va_start, s.va_end));
        n += 1;
    }
    for i in 1..n {
        let key = sorted[i];
        let mut j = i;
        while j > 0 {
            let prev = sorted[j - 1];
            let cur = key;
            if cur.is_none() || prev.is_none() {
                break;
            }
            if prev.unwrap().0 <= cur.unwrap().0 {
                break;
            }
            sorted.swap(j - 1, j);
            j -= 1;
        }
    }
    for w in 1..n {
        let (_a_s, a_e) = sorted[w - 1].unwrap();
        let (b_s, b_e) = sorted[w].unwrap();
        if b_s < a_e {
            return Err(ElfError::SegmentOverlap);
        }
        let _ = b_e;
    }

    // ---- Entry point must land inside an executable segment --------------
    if e_entry % ENTRY_ALIGN != 0 {
        return Err(ElfError::BadEntry);
    }
    let mut entry_ok = false;
    for s in &segments {
        if s.flags & PF_X != 0 && e_entry >= s.va_start && e_entry < s.va_end {
            entry_ok = true;
        }
    }
    if !entry_ok {
        return Err(ElfError::BadEntry);
    }

    Ok(LoadPlan {
        entry: e_entry,
        segments,
    })
}
