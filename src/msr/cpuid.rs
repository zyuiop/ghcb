use super::{GhcbMsr, GhcbMsrInfo};
use super::{MsrRequest, MsrResponse};
use bitfield_struct::{bitenum, bitfield};

impl GhcbMsr {
    pub fn cpuid(function: u32, register: CpuidRegister) -> CpuidResponse {
        // SAFETY: the CPUID function does not modify any state
        unsafe { Self::execute(CpuidRequest::create(function, register)) }
    }
}

#[repr(u8)]
#[bitenum]
#[derive(Debug, PartialEq)]
pub enum CpuidRegister {
    #[fallback]
    Eax = 0b00,
    Ebx = 0b01,
    Ecx = 0b10,
    Edx = 0b11,
}

#[bitfield(u64)]
pub struct CpuidRequest {
    #[bits(30)]
    _reserved: u32,

    #[bits(2)]
    register: CpuidRegister,

    function: u32,
}

impl CpuidRequest {
    pub fn create(function: u32, register: CpuidRegister) -> Self {
        assert_ne!(
            function, 0x0000_000d,
            "0x0000_000d function not supported by CpuID MSR Request"
        );

        Self::new().with_function(function).with_register(register)
    }
}

impl MsrRequest for CpuidRequest {
    type Response = CpuidResponse;

    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::CpuidRequest
    }

    fn data(self) -> u64 {
        self.into_bits()
    }
}

#[bitfield(u64)]
pub struct CpuidResponse {
    #[bits(30)]
    _reserved: u32,

    #[bits(2)]
    register: CpuidRegister,

    function_value: u32,
}

impl MsrResponse for CpuidResponse {
    fn info() -> GhcbMsrInfo {
        GhcbMsrInfo::CpuidResponse
    }

    fn parse(data: u64) -> Self {
        CpuidResponse::from_bits(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let req = CpuidRequest::create(0xC0FFEE, CpuidRegister::Ecx);

        let cpuid_bit = 0b10 << 30;
        assert_eq!(req.data(), 0x00C0FFEE_0000_0000 | cpuid_bit);
    }

    #[test]
    fn test_deserialize() {
        let cpuid_bit = 0b11 << 30;
        let cpuid_response = 0xF00B_AAAA_0000_0000 | cpuid_bit;
        let rep = CpuidResponse::parse(cpuid_response);

        assert_eq!(rep.register(), CpuidRegister::Edx);
        assert_eq!(rep.function_value(), 0xF00B_AAAA)
    }
}
