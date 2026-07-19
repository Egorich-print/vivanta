// ---------------------------------------------------------------------------
// Root page table handle + page mapping flags
// ---------------------------------------------------------------------------

/// Opaque handle to an architecture-specific root page table.
///
/// The vivanta_kernel never inspects the value; each architecture defines how it
/// maps to hardware (AArch64 → TTBR0_EL1, x86_64 → CR3, RISC‑V → SATP).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RootPageTable(pub usize);

/// Opaque page-mapping flags. Bits are defined by each architecture
/// implementation. Kernel constructs flags via named methods only.
#[derive(Clone, Copy, Debug)]
pub struct MappingFlags {
    bits: u64,
}

impl MappingFlags {
    pub const fn from_bits(bits: u64) -> Self {
        Self { bits }
    }

    pub const fn read_write() -> Self {
        Self { bits: 0b001 }
    }

    pub const fn executable() -> Self {
        Self { bits: 0b010 }
    }

    pub const fn user() -> Self {
        Self { bits: 0b100 }
    }

    pub fn is_read_write(&self) -> bool { self.bits & 0b001 != 0 }
    pub fn is_executable(&self) -> bool { self.bits & 0b010 != 0 }
    pub fn is_user(&self) -> bool { self.bits & 0b100 != 0 }
}

impl core::ops::BitOr for MappingFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self { bits: self.bits | rhs.bits }
    }
}

extern "Rust" {
    /// Activate an address space by writing its root page table into the
    /// hardware register (TTBR0_EL1 on AArch64, CR3 on x86_64, SATP on RISC‑V).
    ///
    /// # Safety
    ///
    /// - `root` must point to a valid, fully‑constructed page table.
    /// - The caller must ensure no conflicting translation is live.
    pub fn activate_address_space(root: RootPageTable);
}