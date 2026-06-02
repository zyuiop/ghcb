use super::{GhcbMsrInfo, GhcbMsr};
use super::{MsrRequest, MsrResponse};
use bitfield_struct::bitfield;

pub struct SevInformationRequest;

impl GhcbMsr {
    pub fn get_info() -> SevInformation {
        unsafe { Self::execute(SevInformationRequest) }
    }
}

impl MsrRequest for SevInformationRequest {
    type Response = SevInformation;

    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::SevInformationRequest
    }

    fn data(self) -> u64 {
        0
    }
}

#[bitfield(u64)]
pub struct SevInformation {
    #[bits(24)]
    _padding: u32,

    /// Position of the encryption bit in the physical address
    pub c_bit_pos: u8,
    /// Minimum supported protocol version
    pub min_proto: u16,
    /// Maximum supported protocol version
    pub max_proto: u16,
}

impl MsrResponse for SevInformation {
    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::SevInformation
    }

    fn parse(data: u64) -> Self {
        SevInformation::from_bits(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialization() {
        let value = 0x0300_0200_10_00_0000;
        let resp = SevInformation::parse(value);

        assert_eq!(resp.max_proto(), 0x0300);
        assert_eq!(resp.min_proto(), 0x0200);
        assert_eq!(resp.c_bit_pos(), 0x10);
    }
}
