#![no_std]
#![feature(cfg_select)]

pub mod instructions;
pub mod msr;
pub mod protocols;
pub mod sev_status;
pub mod structures;
pub mod ptr;

#[cfg(feature = "vc-handler")]
pub mod vc_handler;

#[cfg(feature = "console")]
pub mod serial;

#[macro_use]
extern crate bitflags;

#[cfg(all(feature = "alloc", feature = "snp"))]
extern crate alloc;
