use core::{ptr, slice};
use core::mem::MaybeUninit;
use aes_gcm::{AeadInOut, Aes256Gcm, KeyInit, Nonce, Tag};
use aes_gcm::aead::consts::{U12, U16};
use static_assertions::const_assert_eq;
use x86_64::PhysAddr;
use zerocopy::IntoBytes;
use crate::structures::snp_guest_request::{MessageType, SNPAeadAlgorithm, SNPGuestRequest, SNPGuestResponse, SNPHeaderVersion};
use crate::structures::snp_secrets_page::{CommunicationKeyNumber, SecretsPageAccessor, VMCommunicationKey};
use crate::util::OwnedPtrWithPhysAddr;

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
    payload: [u8; PAYLOAD_LEN],
}

const PAYLOAD_LEN: usize = 0x1000 - size_of::<SNPSharedPageHeader>();

impl SNPSharedPage {
    pub fn clear(&mut self) {
        unsafe {
            ptr::from_mut(&mut self.header).write_bytes(0, 1)
        }
    }
    pub fn write_request<SP: SecretsPageAccessor, R: SNPGuestRequest>(&mut self, secrets: &SP, request: R) {
        // Read request as bytes
        let mut request = request;
        let mut request = request.as_mut_bytes();
        let request_size = request.len();

        // Make sure the request is not too large
        assert!(request_size <= PAYLOAD_LEN);

        // Get the next key
        let (key_no, key, seqno) = secrets.with_secrets_page(|page| {
            let (key_no, key, seqno) = page.get_next_available_key().expect("key space exhausted");
            page.increase_sequence_number(key_no);

            (key_no, key, seqno)
        });

        let seqno = seqno + 1;

        // Prepare the header
        self.header.message_type = R::message_type();
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

    pub fn read_response<SP: SecretsPageAccessor, R: SNPGuestResponse>(&self, secrets: &SP) -> R {
        assert_eq!(self.header.message_type, R::message_type(), "invalid response message type");

        assert!(size_of::<R>() <= PAYLOAD_LEN, "response type cannot fit in page");

        let (key, seqno, payload_len) = self.check_response_header::<SP>(secrets);

        assert_eq!(payload_len, size_of::<R>(), "invalid response length received");

        // Decrypt payload
        let mut output_object = unsafe {
            // SAFETY: not safe at this point, but we need to assume it's okay to access the bytes
            MaybeUninit::<R>::uninit().assume_init()
        };
        let mut output_slice = IntoBytes::as_mut_bytes(&mut output_object);

        assert_eq!(payload_len, output_slice.len(), "invalid response length received");

        output_slice.copy_from_slice(&self.payload[..payload_len]);

        let tag = Tag::try_from(&self.header.authentication_tag[0..16]).unwrap();
        let aes = Aes256Gcm::new(&key);

        aes.decrypt_inout_detached(&self.header.nonce(), self.header.associated_data(), output_slice.into(), &tag)
            .expect("failed to decrypt guest protocol response");

        output_object
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

pub trait SharedPageAccessor {
    fn with_shared_page<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&mut OwnedPtrWithPhysAddr<SNPSharedPage>) -> R;
}