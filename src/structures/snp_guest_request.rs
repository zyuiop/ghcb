use core::{ptr, slice};
use aes_gcm::{AeadInOut, Aes256Gcm, KeyInit, Nonce, Tag};
use aes_gcm::aead::consts::{U12, U16};
use static_assertions::const_assert_eq;
use crate::structures::snp_secrets_page::{CommunicationKeyNumber, SecretsPageAccessor, VMCommunicationKey};

#[derive(PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum SNPAeadAlgorithm {
    AesGcm = 1
}

#[derive(PartialEq, Eq, Debug)]
#[repr(u8)]
enum SNPHeaderVersion {
    Version1 = 1
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum MessageType {
    CpuidRequest = 1,
    CpuidResponse = 2,
    KeyRequest = 3,
    KeyResponse = 4,
    ReportRequest = 5,
    ReportResponse = 6,
    ExportRequest = 7,
    ExportResponse = 8,
    ImportRequest = 9,
    ImportResponse = 10,
    AbsorbRequest = 11,
    AbsorbResponse = 12,
    VmrkRequest = 13,
    VmrkResponse = 14,
    AbsorbNomaRequest = 15,
    AbsorbNomaResponse = 16,
    TscInfoRequest = 17,
    TscInfoResponse = 18,
}

#[repr(C)]
struct SNPSharedPageHeader {
    /// Authentication tag for this message
    authentication_tag: [u8; 0x20], // 32 bytes authentication tag

    /// Message sequence number. Used to construct the IV.
    seqno: u64,

    _reserved1: u64,

    /// Algorithm to use to encrypt the message
    algo: SNPAeadAlgorithm,

    /// Header version
    header_version: SNPHeaderVersion,

    /// Header size in bytes
    header_size: u16,

    message_type: MessageType,

    /// Message version - protocol dependent, currently always 1
    message_version: u8,

    /// Payload size in bytes
    payload_size: u16,

    _reserved2: u32,

    /// The key number used for this message. See [`super::super::cc_blob::CommunicationKeyNumber`]
    vmkey: CommunicationKeyNumber,

    _reserved3: u8,
    _reserved4: u16,
    _reserved5: [u8; 0x20],
}

impl SNPSharedPageHeader {
    pub fn nonce(&self) -> Nonce<U12> {
        let mut iv: Nonce<U12> = Default::default();
        iv[0..8].copy_from_slice(self.seqno.to_le_bytes().as_ref());
        iv
    }

    pub fn associated_data(&self) -> &[u8] {
        unsafe {
            let self_ptr = self as *const _ as *const u8;
            let authenticated_data_start = self_ptr.add(0x30);

            // From reference: authenticate bytes 0x30 to 0x5f (inclusive) of the header
            slice::from_raw_parts(authenticated_data_start, 0x30)
        }
    }
}

#[repr(C, align(0x1000))]
pub struct SNPSharedPage {
    header: SNPSharedPageHeader,
    payload: [u8; 0x1000 - size_of::<SNPSharedPageHeader>()],
}

impl SNPSharedPage {
    pub fn clear(&mut self) {
        unsafe {
            ptr::from_mut(&mut self.header).write_bytes(0, 1)
        }
    }
    pub fn write_request<SP: SecretsPageAccessor>(&mut self, secrets: &SP, message_type: MessageType, request: &mut [u8]) {
        // Read request as bytes
        let request_size = request.len();

        // Make sure the request is not too large
        assert!(request_size - size_of::<SNPSharedPageHeader>() <= 0x1000);

        // Get the next key
        let (key_no, key, seqno) = secrets.with_secrets_page(|page| {
            let (key_no, key, seqno) = page.get_next_available_key().expect("key space exhausted");
            page.increase_sequence_number(key_no);

            (key_no, key, seqno)
        });

        let seqno = seqno + 1;

        // Prepare the header
        self.header.message_type = message_type;
        self.header.payload_size = request_size as u16;
        self.header.seqno = seqno as u64;
        self.header.vmkey = key_no;

        self.header.algo = SNPAeadAlgorithm::AesGcm;
        self.header.header_version = SNPHeaderVersion::Version1;
        self.header.header_size = size_of::<SNPSharedPageHeader>() as u16;
        self.header.message_version = 1;

        // Encrypt the payload
        let aes = Aes256Gcm::new(&key);
        let tag: Tag<U16> = aes.encrypt_inout_detached(&self.header.nonce(), self.header.associated_data(), request.into()).unwrap();

        self.header.authentication_tag[0..16].copy_from_slice(tag.as_slice());
        self.payload[0..request_size].clone_from_slice(request);
    }

    fn check_response_header<SP: SecretsPageAccessor>(&self, secrets: &SP) -> (VMCommunicationKey, u32, usize) {
        // Verify header
        assert_eq!(self.header.header_version, SNPHeaderVersion::Version1);
        assert_eq!(self.header.algo, SNPAeadAlgorithm::AesGcm);
        assert_eq!(self.header.header_size, size_of::<SNPSharedPageHeader>() as u16);

        let payload_len = self.header.payload_size as usize;
        assert_ne!(payload_len, 0);
        assert!(payload_len < self.payload.len());

        let key_used = CommunicationKeyNumber::try_from(self.header.vmkey).expect("invalid VMKey number");
        let (key, seqno) = secrets.with_secrets_page(|page| {
            page.increase_sequence_number(key_used);
            let seqno = page.get_sequence_number(key_used);
            let key = page.get_key(key_used);

            (key, seqno)
        });

        assert_eq!(self.header.seqno, seqno as u64);

        (key, seqno, payload_len)
    }

    /// Reads a binary response from the secrets page and returns the number of bytes read
    pub fn read_response_raw<SP: SecretsPageAccessor>(&self, secrets: &SP, output: &mut [u8; 0x1000 - size_of::<SNPSharedPageHeader>()]) -> usize {
        let (key, seqno, payload_len) = self.check_response_header::<SP>(secrets);

        // Decrypt payload
        output.copy_from_slice(&self.payload);

        let output = &mut output[..payload_len];

        let tag = Tag::try_from(&self.header.authentication_tag[0..16]).unwrap();
        let aes = Aes256Gcm::new(&key);

        aes.decrypt_inout_detached(&self.header.nonce(), self.header.associated_data(), output.into(), &tag)
            .expect("failed to decrypt guest protocol response");

        payload_len
    }

    /// Reads a binary response from the secrets page and returns a vector for the bytes read
    #[cfg(feature = "alloc")]
    pub fn read_response_raw_to_vec<SP: SecretsPageAccessor>(&self, secrets: &SP) -> alloc::vec::Vec<u8> {
        let (key, seqno, payload_len) = self.check_response_header::<SP>(secrets);

        // Decrypt payload
        let mut vec = alloc::vec::Vec::from(&self.payload[0..payload_len]);
        let tag = Tag::try_from(&self.header.authentication_tag[0..16]).unwrap();
        let aes = Aes256Gcm::new(&key);

        aes.decrypt_inout_detached(&self.header.nonce(), self.header.associated_data(), vec.as_mut_slice().into(), &tag)
            .expect("failed to decrypt guest protocol response");

        vec
    }

}

const_assert_eq!(size_of::<SNPSharedPage>(), 4096);
