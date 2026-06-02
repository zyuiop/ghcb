#![no_std]

pub mod msr;
pub mod structures;
pub mod protocols;

#[macro_use]
extern crate bitflags;

#[cfg(feature = "alloc")]
extern crate alloc;

use core::arch::asm;

/// Exits to the hypervisor to execute a GHCB request.
///
/// ## Safety
///
/// The GHCB MSR must be set to a valid value.
/// The GHCB page must contain valid content.
/// The hypervisor will modify the state of the GHCB page and/or registers.
#[inline(always)]
pub(crate) unsafe fn vmgexit() {
    unsafe {
        asm!("rep; vmmcall\n\r", options());
    }
}
