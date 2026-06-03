//! This module contains good-enough handlers for some common VC exits.
pub mod handler_ioio;

#[cfg(feature = "snp")]
pub mod handler_cpuid;
