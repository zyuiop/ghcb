use crate::msr::{GhcbMsr, GhcbMsrInfo, MsrRequest, MsrResponse};

pub struct HypervisorFeatureSupportRequest;

impl GhcbMsr {
    pub fn get_features() -> HypervisorFeatures {
        unsafe { Self::execute(HypervisorFeatureSupportRequest) }
    }
}

impl MsrRequest for HypervisorFeatureSupportRequest {
    type Response = HypervisorFeatures;

    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::HypervisorFeatureSupportRequest
    }

    fn data(self) -> u64 {
        0
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug)]
    /// The features beatmap supported by an hypervisor.
    /// Refer to GHCB standard (AMD document 56421), Table 1
    pub struct HypervisorFeatures: u64 {
        /// Support for base SEV-SNP features
        const SEV_SNP = 0;

        /// Support for AP VMSA creation / AP Create NAE event
        const SEV_SNP_AP_CREATION = 1;

        /// Support for SEV-SNP restricted injection
        const SEV_SNP_RESTRICTED_INJECTION = 2;

        /// Support for SEV-SNP restricted injection timer
        const SEV_SNP_RESTRICTED_INJECTION_TIMER = 3;

        /// Supports returning the list of APIC IDs associated with guest vCPUS
        const APIC_ID_LIST = 4;

        /// Supports running different vCPUs at different VMPL levels
        const SEV_SNP_MULTI_VMPL = 5;

        /// Supports the [super::page_state_change::PageStateChangeRequest] and response protocol,
        /// and the associated GHCB protocol.
        const SEV_ES_PAGE_STATE_CHANGE = 6;

        /// Supports Trusted IO devices
        const SEV_TIO = 7;

        /// Supports unregistering GHCB GPAs
        const GHCB_UNREGISTER = 8;
    }
}

impl MsrResponse for HypervisorFeatures {
    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::HypervisorFeatureSupportResponse
    }

    fn parse(data: u64) -> Self {
        HypervisorFeatures::from_bits_truncate(data >> 12)
    }
}
