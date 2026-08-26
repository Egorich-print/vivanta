#![no_std]
//! ELF64 AArch64 program image parsing and load planning (M8.1/M8.2
//! contract layer, G-M8).
//!
//! This module is PURE: it reads an in-memory image and produces a
//! validated [`LoadPlan`] — the complete description of *what* to map,
//! copy, zero-fill and where to enter. It never touches page tables or
//! physical memory; the kernel-side loader executes the plan through VMM
//! primitives (ADR-031/032 boundaries preserved).
//!
//! Security posture: every field is bounds-checked against the image
//! before use; integer arithmetic is overflow-checked; segments must be
//! disjoint after page-rounding (overlapping PT_LOADs are a classic
//! loader-exploit primitive); W^X policy is enforced at plan time.

pub mod elf;

pub use elf::{ENTRY_ALIGN, ElfError, LoadPlan, LoadSegment};

#[cfg(test)]
mod elf_tests;
