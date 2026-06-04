use aes_gcm::Key;
use aes_gcm::aes::Aes256;
use static_assertions::const_assert_eq;
use zerocopy_derive::FromZeros;

pub trait SecretsPageAccessor {
    fn with_secrets_page<F, R>(&self, func: F) -> R
    where
        F: Fn(&mut SNPSecretsPage) -> R;
}

pub type VMCommunicationKey = Key<Aes256>;

#[derive(Copy, Clone, Debug, FromZeros)]
#[repr(u8)]
pub enum CommunicationKeyNumber {
    VmPck0 = 0,
    VmPck1 = 1,
    VmPck2 = 2,
    VmPck3 = 3,
}

impl From<CommunicationKeyNumber> for usize {
    fn from(value: CommunicationKeyNumber) -> Self {
        (value as u8) as usize
    }
}

#[derive(Debug)]
#[repr(C)]
struct OSSecretsArea {
    msg_seqno: [u32; 4],
    ap_jump_table_phys_addr: u64,
    reserved: [u8; 40],
    guest_usage: [u8; 20],
    _padding: [u8; 12],
}

#[derive(Debug)]
#[repr(C)]
pub struct SNPSecretsPage {
    version: u32,
    imi_en: u32,
    fms: u32,
    _reserved: u32,
    gosvw: [u8; 16],
    vmpck: [VMCommunicationKey; 4],
    guest_area: OSSecretsArea,
    vmsa_tweak_bitmap: [u8; 64],
    guest_area_2: [u8; 32],
    tsc_factor: u32,
    _reserved2: u32,
    launch_mit_vector: u64,
    _reserved3: [u8; 3728],
}

impl SNPSecretsPage {
    #[inline]
    pub fn get_key(&self, no: CommunicationKeyNumber) -> VMCommunicationKey {
        self.vmpck[usize::from(no)]
    }

    #[inline]
    pub fn get_sequence_number(&self, no: CommunicationKeyNumber) -> u32 {
        self.guest_area.msg_seqno[usize::from(no)]
    }

    #[inline]
    pub fn increase_sequence_number(&mut self, no: CommunicationKeyNumber) {
        self.guest_area.msg_seqno[usize::from(no)] += 1;
    }

    pub fn get_next_available_key(
        &self,
    ) -> Option<(CommunicationKeyNumber, VMCommunicationKey, u32)> {
        for key_no in [
            CommunicationKeyNumber::VmPck0,
            CommunicationKeyNumber::VmPck1,
            CommunicationKeyNumber::VmPck2,
            CommunicationKeyNumber::VmPck3,
        ] {
            let seqno = self.guest_area.msg_seqno[usize::from(key_no)];

            if seqno < u32::MAX - 1 {
                let key = self.vmpck[usize::from(key_no)];
                return Some((key_no, key, seqno));
            }
        }

        None
    }
}

const_assert_eq!(size_of::<OSSecretsArea>(), 96);
const_assert_eq!(size_of::<SNPSecretsPage>(), 4096);
