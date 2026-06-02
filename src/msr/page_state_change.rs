use crate::msr::{GhcbMsrInfo, MsrRequest, MsrResponse};
use bitfield_struct::{bitenum, bitfield};
use x86_64::structures::paging::{PhysFrame, Size4KiB};

#[repr(u8)]
#[bitenum]
#[derive(Debug, PartialEq)]
pub enum PageStateOperation {
    #[fallback]
    AssignPrivate = 0x01,
    AssignShared = 0x02,
}

#[bitfield(u64)]
pub struct PageStateChangeRequest {
    #[bits(12)]
    _info: u16,
    #[bits(40)]
    frame_number: u64,
    #[bits(4)]
    operation: PageStateOperation,
    _reserved: u8,
}

impl PageStateChangeRequest {
    pub fn create(frame: PhysFrame<Size4KiB>, operation: PageStateOperation) -> Self {
        Self::new()
            .with_frame_number(frame.start_address().as_u64() >> 12)
            .with_operation(operation)
    }
}

impl MsrRequest for PageStateChangeRequest {
    type Response = PageStateChangeResponse;

    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::PageStateChangeRequest
    }

    fn data(self) -> u64 {
        self.into_bits()
    }
}

/// The response for a page state change request. Should be converted to a Result and used.
#[must_use]
pub struct PageStateChangeResponse(Option<u64>);

impl MsrResponse for PageStateChangeResponse {
    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::PageStateChangeResponse
    }

    fn parse(data: u64) -> Self {
        Self(Some(data >> 12).filter(|&data| data != 0))
    }
}

impl From<PageStateChangeResponse> for Result<(), u64> {
    fn from(value: PageStateChangeResponse) -> Self {
        match value.0 {
            None => Ok(()),
            Some(code) => Err(code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x86_64::PhysAddr;

    #[test]
    fn test_serialize() {
        let req = PageStateChangeRequest::create(
            PhysFrame::from_start_address(PhysAddr::new(0x7B0F_4000)).unwrap(),
            PageStateOperation::AssignShared,
        );
        let state_change_bit = 0x2 << 52;

        assert_eq!(req.data(), 0x00_00_0000_7B0F_4000 | state_change_bit);
    }
}
