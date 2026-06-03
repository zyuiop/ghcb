use crate::structures::snp_guest_request::{MessageType, SNPGuestRequest, SNPGuestResponse};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
enum KeySelection {
    /// Sign with VLEK if available, otherwise sign with VCEK
    VLEKIfAvailable = 0,
    VCEK = 1,
    VLEK = 2,
}

#[derive(Copy, Clone, FromBytes, IntoBytes, Immutable, Debug)]
#[repr(C)]
pub struct AttestationRequest {
    report_data: [u8; 0x40],
    vmpl: u32,
    _reserved: [u8; 3],

    // Formally, only the last two bits of this are used
    /// See: [Self::KeySelection]
    key_sel: u8,

    _reserved2: [u8; 24],
}

impl AttestationRequest {
    pub fn new(report_data: Option<[u8; 0x40]>) -> Self {
        Self {
            report_data: report_data.unwrap_or([0u8; 0x40]),
            vmpl: 0,
            _reserved: [0; 3],
            _reserved2: [0; 24],
            key_sel: KeySelection::VLEKIfAvailable as u8,
        }
    }
}

impl SNPGuestRequest for AttestationRequest {
    type ResponseType = AttestationResponse;

    fn message_type() -> MessageType {
        MessageType::ReportRequest
    }
}

const REPORT_SIZE: usize = 1184usize;

#[repr(C)]
#[derive(IntoBytes, FromBytes, Debug)]
pub struct AttestationResponse {
    status: u32,
    report_size: u32,
    _reserved: [u8; 24],
    report: [u8; REPORT_SIZE],
}

impl SNPGuestResponse for AttestationResponse {
    fn message_type() -> MessageType {
        MessageType::ReportResponse
    }
}
