#![no_std]

//! Vivanta VM mechanisms — architecture-neutral, hardware-free.
//!
//! Everything in this crate is pure bookkeeping with total functions:
//! every operation either succeeds deterministically or returns an error
//! without mutating state. Hardware programming lives in `arch-*` crates;
//! permission policy lives in the encoders (`ap_bits`). See ADR-031
//! (planned) for the ownership/lifecycle model.

pub mod va;

#[cfg(test)]
mod va_tests;

pub use va::{PAGE_SIZE, VaAllocator, VaError, VaRegion};
