use crate::vc_handler::VcHandler;
use crate::vc_handler::structures::instruction_parser::InstructionData;
use crate::vc_handler::structures::stack_frame::VCInterruptStackFrame;
use x86_64::registers::control::Cr2;
#[cfg(feature = "logging")]
use x86_64::structures::paging::Translate;
#[cfg(feature = "logging")]
use x86_64::structures::paging::mapper::TranslateResult;

#[cfg(feature = "logging")]
pub struct RmpInvalidHandler<'a, T: Translate>(&'a T);

#[cfg(not(feature = "logging"))]
pub struct RmpInvalidHandler;

#[cfg(feature = "logging")]
impl<'a, T: Translate> RmpInvalidHandler<'a, T> {
    #[inline(always)]
    pub const fn new(handler: &'a T) -> Self {
        Self(handler)
    }
}

#[cfg(not(feature = "logging"))]
impl VcHandler for RmpInvalidHandler {
    fn handle(&self, _: &mut VCInterruptStackFrame, _: &mut InstructionData) {
        let va = Cr2::read().unwrap();
        panic!("Invalid RMP entry when accessing 0x{va:x}");
    }
}
#[cfg(feature = "logging")]
impl<'a, T: Translate> VcHandler for RmpInvalidHandler<'a, T> {
    fn handle(&self, _: &mut VCInterruptStackFrame, _: &mut InstructionData) {
        // See AMD Programmer's Manual vol. 2 (doc id 24593), section §15.36.10
        // > A failure of the page validation check results in a #VC with error code PAGE_NOT_VALIDATED
        // > (0x404). The faulting guest virtual address is saved to CR2 when this error occurs.

        /* The VM may use the PVALIDATE instruction to either set or clear the Validated flag of a page. It is
        expected that VMs would use PVALIDATE to set the Validated flag during VM startup to gain access
        to the memory the hypervisor has assigned. The VM may later use PVALIDATE to clear the Validated
        flag if its memory space is being reduced, such as after a memory hot-plug event.
        Page validation allows a VM to detect an unexpected remapping of its pages by the hypervisor. Before
        accessing a page, the VM must validate the page. Once validated, any use of RMPUPDATE by the
        hypervisor to unassign, reassign, or remap the page will cause the page to become unvalidated. The
        VM can then detect tampering with the page mapping via the #VC that occurs from accessing
        unvalidated pages. */

        let va = Cr2::read().unwrap();

        log::error!("Invalid RMP entry when accessing 0x{va:x}");

        let translate_result = self.0.translate(va);

        match translate_result {
            TranslateResult::NotMapped | TranslateResult::InvalidFrameAddress(_) => {
                log::error!("Has no page table mapping");
            }
            TranslateResult::Mapped {
                frame,
                offset,
                flags,
            } => {
                log::error!(" -> Is mapped to physical frame: {:?}", frame);
                log::error!(" -> Offset within frame: {:x}", offset);
                log::error!(" -> Flags: {:?}", flags);
            }
        };

        panic!("Invalid RMP entry when accessing 0x{va:x}");
    }
}
