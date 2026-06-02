use crate::msr::{GhcbMsrInfo, GhcbMsr, MsrRequest, MsrResponse};
use bitfield_struct::bitfield;
use core::arch::asm;

#[bitfield(u16)]
pub struct TerminationRequest {
    #[bits(4)]
    reason_code_set: u8,
    reason_code: u8,
    #[bits(4)]
    _padding: u8,
}

impl GhcbMsr {
    pub fn terminate(code_set: u8, code: u8) -> ! {
        assert!(code_set <= 0xf, "code_set must be lower than 16!");

        unsafe {
            Self::execute(
                TerminationRequest::new()
                    .with_reason_code_set(code_set)
                    .with_reason_code(code),
            );
        }

        // In case the termination returns, we enter an infinite halt loop
        loop {
            unsafe { asm!("hlt") }
        }
    }
}

impl MsrRequest for TerminationRequest {
    type Response = NoResponse;

    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::TerminationRequest
    }

    fn data(self) -> u64 {
        (self.into_bits() as u64) << 12
    }
}

pub struct NoResponse;

impl MsrResponse for NoResponse {
    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::TerminationRequest
    }

    fn parse(_: u64) -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let request = TerminationRequest::new()
            .with_reason_code_set(0x7)
            .with_reason_code(0x42)
            .data();

        assert_eq!(request, 0x0000_0000_0042_7000)
    }
}
