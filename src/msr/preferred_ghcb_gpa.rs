use crate::msr::{GhcbMsrInfo, GhcbMsr};
use crate::msr::{MsrRequest, MsrResponse};
use x86_64::PhysAddr;
use x86_64::structures::paging::{PhysFrame, Size4KiB};

pub struct PreferredGhcbGpaRequest;

impl GhcbMsr {
    pub fn get_preferred_ghcb_gpa() -> Option<PhysFrame<Size4KiB>> {
        unsafe { Self::execute(PreferredGhcbGpaRequest).addr() }
    }
}

impl MsrRequest for PreferredGhcbGpaRequest {
    type Response = PreferredGhcbGpaResponse;

    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::PreferredGhcbGpaRequest
    }

    fn data(self) -> u64 {
        0
    }
}

pub struct PreferredGhcbGpaResponse(Option<PhysFrame<Size4KiB>>);

impl MsrResponse for PreferredGhcbGpaResponse {
    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::PreferredGhcbGpaResponse
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

impl PreferredGhcbGpaResponse {
    pub fn addr(&self) -> Option<PhysFrame<Size4KiB>> {
        self.0
    }
}
