use crate::msr::GhcbMsrInfo;
use crate::msr::{MsrRequest, MsrResponse};

pub struct ApResetHoldRequest;

impl MsrRequest for ApResetHoldRequest {
    type Response = ApResetHoldResponse;

    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::ApResetHoldRequest
    }

    fn data(self) -> u64 {
        0
    }
}

pub enum ApResetHoldResponse {
    /// The AP reset hold sequence was a success
    Success,

    /// The AP reset hold sequence failed and must be retried later
    RetryLater,
}

impl MsrResponse for ApResetHoldResponse {
    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::ApResetHoldResponse
    }

    fn parse(data: u64) -> Self {
        if data == 0 {
            ApResetHoldResponse::RetryLater
        } else {
            ApResetHoldResponse::Success
        }
    }
}
