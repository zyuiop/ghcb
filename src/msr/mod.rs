pub mod ap_reset_hold;
pub mod cpuid;
pub mod hypervisor_features;
pub mod page_state_change;
pub mod preferred_ghcb_gpa;
pub mod register_ghcb_gpa;
pub mod sev_info;
#[cfg(feature = "snp")]
pub mod snp_run_vmpl;
pub mod terminate;
pub mod unregister_ghcb_gpa;

use crate::instructions::vmgexit;
use bitfield_struct::{bitenum, bitfield};
use x86_64::PhysAddr;
use x86_64::registers::model_specific::Msr;
use x86_64::structures::paging::{PhysFrame, Size4KiB};

pub trait MsrRequest {
    type Response: MsrResponse;

    /// Return the Info part for this request
    fn info() -> GhcbMsrInfo;

    /// Return the data part for this request, with the last 12 bits set to 0 (for Info)
    fn data(self) -> u64;
}

pub trait MsrResponse {
    /// The info part of the request that should be returned by the hypervisor.
    fn info() -> GhcbMsrInfo;

    /// Parse the data part of this request.
    /// `data` is a 64bit integer with the 12 least significant bits set to 0.
    fn parse(data: u64) -> Self;
}

#[derive(Debug)]
pub struct GhcbMsr;

impl GhcbMsr {
    const fn msr() -> Msr {
        Msr::new(0xC001_0130)
    }

    fn write(data: GhcbMsrData) {
        unsafe { Self::msr().write(data.into_bits()) }
    }

    fn read() -> GhcbMsrData {
        unsafe { GhcbMsrData::from_bits(Self::msr().read()) }
    }

    /// Writes the GHCB address in the GHCB MSR.
    ///
    /// ## Safety
    ///
    /// The address must correspond to a frame shared with the hypervisor. Its content will
    /// be interpreted as a [Ghcb] on the next [vmmexit].
    /// The address must have been previously registered using the [RegisterGhcbAddressProtocol] in
    /// AMD SEV-SNP
    pub unsafe fn set_ghcb_address(frame: PhysFrame<Size4KiB>) {
        unsafe {
            // SAFETY: frame start addresses always end with the last 12 bits set to 0, so this
            // is a valid call
            Self::msr().write(frame.start_address().as_u64())
        }
    }

    /// Ensures that the GHCB address set in the MSR matches the passed argument, calling
    /// [Self::set_ghcb_address] if that's not the case.
    ///
    /// ## Safety
    ///
    /// The address must correspond to a frame shared with the hypervisor. Its content will
    /// be interpreted as a [Ghcb] on the next [vmmexit].
    /// The address must have been previously registered using the [RegisterGhcbAddressProtocol] in
    /// AMD SEV-SNP
    pub unsafe fn ensure_ghcb_address_is(frame: PhysFrame<Size4KiB>) {
        if Self::get_current_ghcb_address().is_none_or(|f| f != frame) {
            unsafe {
                Self::set_ghcb_address(frame);
            }
        }
    }

    /// Returns the address currently set as the GHCB address in the MSR
    pub fn get_current_ghcb_address() -> Option<PhysFrame<Size4KiB>> {
        let read = Self::read();
        if read.ghcb_info() != GhcbMsrInfo::GhcbGuestPhysicalAddress {
            return None;
        }

        let address = PhysAddr::new(read.0);
        PhysFrame::from_start_address(address).ok()
    }

    /// Executes a GHCB MSR request, then resets the GHCB address if it was present, and returns
    /// the response.
    ///
    /// ## Safety
    ///
    /// The request may change the state of the VM
    pub unsafe fn execute<T: MsrRequest>(request: T) -> T::Response {
        let previous_value = Self::read();

        // Build and write the request
        Self::write(
            GhcbMsrData::new()
                .with_ghcb_info(T::info())
                .with_data(request.data() >> 12),
        );

        // Jump to hypervisor
        unsafe {
            vmgexit();
        }

        // Read and parse response
        let response = Self::read();
        assert_eq!(response.ghcb_info(), T::Response::info());
        let response = T::Response::parse(response.data() << 12);

        // Restore previous value.
        // It may be a previous response/request from an interrupted [execute] call, or it may
        // be the current registered GHCB address. In any case, we want to leave the register
        // untouched after execute.
        Self::write(previous_value);

        response
    }
}

#[bitfield(u64)]
pub(crate) struct GhcbMsrData {
    #[bits(12)]
    pub(crate) ghcb_info: GhcbMsrInfo,

    #[bits(52)]
    pub(crate) data: u64,
}

#[repr(u16)]
#[bitenum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GhcbMsrInfo {
    GhcbGuestPhysicalAddress = 0x000,
    SevInformation = 0x001,
    SevInformationRequest = 0x002,
    CpuidRequest = 0x004,
    CpuidResponse = 0x005,
    ApResetHoldRequest = 0x006,
    ApResetHoldResponse = 0x007,
    PreferredGhcbGpaRequest = 0x010,
    PreferredGhcbGpaResponse = 0x011,
    RegisterGhcbGpaRequest = 0x012,
    RegisterGhcbGpaResponse = 0x013,
    PageStateChangeRequest = 0x014,
    PageStateChangeResponse = 0x015,

    #[cfg(feature = "snp")]
    SnpRunVmplRequest = 0x016,
    #[cfg(feature = "snp")]
    SnpRunVmplResponse = 0x017,

    UnregisterGhcbGpaRequest = 0x018,
    UnregisterGhcbGpaResponse = 0x019,

    HypervisorFeatureSupportRequest = 0x080,
    HypervisorFeatureSupportResponse = 0x081,

    TerminationRequest = 0x100,

    #[fallback]
    Invalid = 0x099,
}

#[cfg(test)]
mod tests {
    use crate::msr::{GhcbMsrData, GhcbMsrInfo};

    #[test]
    fn test_serialize() {
        let data = GhcbMsrData::new()
            .with_ghcb_info(GhcbMsrInfo::HypervisorFeatureSupportRequest)
            .with_data(0xABCDEF);

        assert_eq!(data.into_bits(), 0x0000_000A_BCDE_F080)
    }
}
