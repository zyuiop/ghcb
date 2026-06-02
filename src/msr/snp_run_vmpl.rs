use crate::msr::{GhcbMsrInfo, MsrRequest, MsrResponse};

pub struct SnpRunVmplRequest(u8);

impl SnpRunVmplRequest {
    pub fn create(vmpl: u8) -> Self {
        assert!(vmpl <= 3, "VMPL must be <= 3");
        SnpRunVmplRequest(vmpl)
    }
}

impl MsrRequest for SnpRunVmplRequest {
    type Response = SnpRunVmplResponse;

    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::SnpRunVmplRequest
    }

    fn data(self) -> u64 {
        (self.0 as u64) << 32
    }
}

/// The response for a [SnpRunVmplRequest]. Should be converted to a Result and used.
#[must_use]
pub struct SnpRunVmplResponse(Option<u64>);

impl MsrResponse for SnpRunVmplResponse {
    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::SnpRunVmplResponse
    }

    fn parse(data: u64) -> Self {
        Self(Some(data >> 12).filter(|&data| data != 0))
    }
}

impl From<SnpRunVmplResponse> for Result<(), u64> {
    fn from(value: SnpRunVmplResponse) -> Self {
        match value.0 {
            None => Ok(()),
            Some(code) => Err(code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let req = SnpRunVmplRequest::create(0x1);
        assert_eq!(req.data(), 0x0000_0001_0000_0000);
    }
}
