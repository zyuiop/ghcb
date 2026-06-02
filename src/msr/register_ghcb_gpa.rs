use crate::msr::{GhcbMsrInfo, GhcbMsr};
use crate::msr::{MsrRequest, MsrResponse};
use x86_64::PhysAddr;
use x86_64::structures::paging::{PhysFrame, Size4KiB};

#[derive(Debug)]
pub struct RegisterGhcbGpaRequest(PhysFrame<Size4KiB>);

#[derive(Debug)]
pub struct RegistrationFailed;

impl core::fmt::Display for RegistrationFailed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Failed to register GHCB with hypervisor")
    }
}

impl core::error::Error for RegistrationFailed {}

impl GhcbMsr {
    /// Registers a provided frame as the GHCB address using the Ghcb registration protocol MSR,
    /// then sets that address as the GHCB address in the MSR.
    ///
    /// ## Safety
    ///
    /// This method calls [Self::register_ghcb], so the frame must contain a valid GHCB frame,
    /// shared with the hypervisor.
    ///
    /// This method will unregister any previously registered GHCB, which is dangerous.
    pub unsafe fn register_ghcb(frame: PhysFrame<Size4KiB>) -> Result<(), RegistrationFailed> {
        let response = unsafe { Self::execute(RegisterGhcbGpaRequest::new(frame)) };

        if response
            .addr()
            .is_none_or(|return_frame| return_frame != frame)
        {
            // Hypervisor changed the GFN or returned 0
            return Err(RegistrationFailed);
        }

        unsafe {
            Self::set_ghcb_address(frame);
        }

        Ok(())
    }
}

impl RegisterGhcbGpaRequest {
    pub fn new(frame: PhysFrame<Size4KiB>) -> Self {
        Self(frame)
    }
}

impl MsrRequest for RegisterGhcbGpaRequest {
    type Response = RegisterGhcbGpaResponse;

    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::RegisterGhcbGpaRequest
    }

    fn data(self) -> u64 {
        self.0.start_address().as_u64()
    }
}

#[must_use]
pub struct RegisterGhcbGpaResponse(Option<PhysFrame<Size4KiB>>);

impl MsrResponse for RegisterGhcbGpaResponse {
    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::RegisterGhcbGpaResponse
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

impl RegisterGhcbGpaResponse {
    /// Returns the address returned by the hypervisor. Callers must check that the address
    /// matches the requested address.
    /// If none, the hypervisor rejected the address
    pub fn addr(&self) -> Option<PhysFrame<Size4KiB>> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let req = RegisterGhcbGpaRequest(
            PhysFrame::from_start_address(PhysAddr::new(0x7B0F_4000)).unwrap(),
        );

        assert_eq!(req.data(), 0x7B0F_4000)
    }

    #[test]
    fn test_deserialize() {
        let req = RegisterGhcbGpaResponse::parse(0x7B0F_4000);
        let addr = req.addr().unwrap();
        assert_eq!(addr.start_address().as_u64(), 0x7B0F_4000)
    }
}
