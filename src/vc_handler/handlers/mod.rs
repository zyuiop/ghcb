//! This module contains good-enough handlers for some common VC exits.
pub mod handler_ioio;

#[cfg(feature = "snp")]
pub mod handler_cpuid;
pub mod handler_mmio;
pub mod handler_msr;
pub mod handler_rmp_invalid;
