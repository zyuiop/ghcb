pub mod change_page_state;
pub mod cpuid;
pub mod ioio;
pub mod mmio;
pub mod msr;
#[cfg(feature = "snp")]
pub mod snp_ap_create;
#[cfg(feature = "snp")]
pub mod snp_guest_request;
pub mod unsupported;

use crate::instructions::vmgexit;
use crate::structures::ChannelManager;
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::errors::MalformedGhcbError;
use crate::structures::exit_codes::GhcbExitCode;
use crate::structures::ghcb_page::GhcbU64Field;
use bitfield_struct::{bitenum, bitfield};

pub trait GhcbProtocolRequest: Sized {
    type Response;

    /// Execute this request with an already acquired GHCB
    fn execute_request(self, ghcb: &mut GhcbRequestExecutor) -> Self::Response;

    /// Acquires a GHCB using the passed ChannelManager, and executes
    /// this request using it.
    ///
    /// Warning: if you already have a GHCB, you should use [Self::execute_request] instead.
    fn execute<T: ChannelManager>(self) -> Self::Response {
        let ghcb = T::get_channel();
        ghcb.with_ghcb(|mut ghcb| self.execute_request(&mut ghcb))
    }
}

impl GhcbRequestExecutor<'_> {
    pub(crate) fn checked_vmgexit(
        &mut self,
        exitcode: GhcbExitCode,
        exit_info1: u64,
        exit_info2: u64,
    ) {
        let ghcb = self.raw();

        ghcb.set_exit_code(exitcode);
        ghcb.set_field(GhcbU64Field::SwExitInfo1, exit_info1);
        ghcb.set_field(GhcbU64Field::SwExitInfo2, exit_info2);

        unsafe {
            vmgexit();
        }

        // Check error code if present
        match ghcb
            .get_field_if_valid(GhcbU64Field::SwExitInfo1)
            .expect("Invalid vmgexit result")
            & 0xffff_ffff
        {
            0x0000 => (),
            0x0001 => {
                let exit2 = ghcb
                    .get_field_if_valid(GhcbU64Field::SwExitInfo2)
                    .expect("Missing event injection details");
                let inject = EventInjection::from_bits(exit2);

                handle_event_injection(inject);
            }
            0x0002 => {
                let exit2 = ghcb
                    .get_field_if_valid(GhcbU64Field::SwExitInfo2)
                    .expect("Missing error code");
                let error = MalformedGhcbError::from(exit2);
                panic!("GHCB Protocol Error: {error}");
            }
            other => {
                panic!("GHCB Protocol Error - non-zero ExitInfo2 {other:x}");
            }
        }
    }
}

#[bitenum]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventInjectionType {
    #[fallback]
    INTR = 0x0,
    NMI = 0x2,
    Exception = 0x3,
    SoftwareInterrupt = 0x4,
}

#[bitfield(u64)]
struct EventInjection {
    vector: u8,
    #[bits(3)]
    typ: EventInjectionType,
    #[bits(1)]
    error_code_valid: bool,
    #[bits(19)]
    _reserved: u32,
    #[bits(1)]
    valid: bool,
    error_code: u32,
}

fn handle_event_injection(event_injection: EventInjection) {
    panic!("Unhandled event injection request: {:?}", event_injection);
}
