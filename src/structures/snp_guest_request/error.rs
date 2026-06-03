use crate::structures::snp_guest_request::GuestProtocolStatusCode;

#[derive(Debug)]
pub enum GuestProtocolError {
    CryptoError,
    FirmwareError {
        hv_error: u32,
        fw_error: GuestProtocolStatusCode,
    }
}

impl GuestProtocolError {
    pub fn from_fw_error(err: u64) -> GuestProtocolError {
        GuestProtocolError::FirmwareError {
            hv_error: (err >> 32) as u32,
            fw_error: GuestProtocolStatusCode::from_bits((err & 0xffff_ffff) as u32)
        }
    }
}
