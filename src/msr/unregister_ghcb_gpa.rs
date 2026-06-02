use crate::msr::GhcbMsrInfo;
use crate::msr::preferred_ghcb_gpa::PreferredGhcbGpaResponse;
use crate::msr::{MsrRequest, MsrResponse};
use x86_64::PhysAddr;
use x86_64::structures::paging::{PhysFrame, Size4KiB};

pub struct UnegisterGhcbGpaRequest;

impl MsrRequest for UnegisterGhcbGpaRequest {
    type Response = PreferredGhcbGpaResponse;

    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::UnregisterGhcbGpaRequest
    }

    fn data(self) -> u64 {
        0
    }
}

pub struct UnregisterGhcbGpaResponse(Option<PhysFrame<Size4KiB>>);

impl MsrResponse for UnregisterGhcbGpaResponse {
    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::UnregisterGhcbGpaResponse
    }

    fn parse(data: u64) -> Self {
        let addr = Some(data & 0xffff_ffff_ffff_f000)
            .filter(|&data| data != 0xffff_ffff_ffff_f000)
            .map(PhysAddr::new)
            .map(|data| unsafe {
                // SAFETY: correct address alignment is ensured by definition (last 12 bits set to 0)
                PhysFrame::from_start_address_unchecked(data)
            });

        Self(addr)
    }
}

impl UnregisterGhcbGpaResponse {
    /// Returns the address of the previously registered GHCB page
    pub fn addr(&self) -> Option<PhysFrame<Size4KiB>> {
        self.0
    }
}
