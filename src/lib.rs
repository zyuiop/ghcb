#![no_std]
#![feature(cfg_select)]

pub mod msr;
pub mod structures;
pub mod protocols;
pub mod sev_status;
pub mod util;
pub mod instructions;

#[cfg(feature = "vc-handler")]
pub mod vc_handler;

#[macro_use]
extern crate bitflags;

#[cfg(feature = "alloc")]
extern crate alloc;

