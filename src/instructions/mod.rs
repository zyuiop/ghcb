pub mod pvalidate;
pub mod rmpadjust;

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
