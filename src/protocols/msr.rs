use core::marker::PhantomData;
use crate::protocols::GhcbProtocolRequest;
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::ChannelManager;
use crate::structures::exit_codes::GhcbExitCode;
use crate::structures::ghcb_page::GhcbU64Field;

#[derive(Debug)]
#[repr(transparent)]
pub struct GhcbMsr<T> {
    msr: u32,
    _phandom: PhantomData<T>
}

impl<T: ChannelManager> GhcbMsr<T> {
    pub const fn new(msr: u32) -> Self {
        Self { msr, _phandom: PhantomData }
    }

    #[inline]
    pub unsafe fn read(&self) -> u64 {
        ReadMsrRequest { msr: self.msr }.execute_to_u64::<T>()
    }

    #[inline]
    pub unsafe fn write(&mut self, value: u64) {
        WriteMsrRequest::from_value(self.msr, value).execute::<T>()
    }
}

pub struct WriteMsrRequest {
    msr: u32,
    msb: u32,
    lsb: u32
}

impl WriteMsrRequest {
    /// Create a MSR write request from raw register values.
    /// In a wrmsr request, `msb` is in rdx, and `lsb` is in rax.
    #[inline]
    pub fn new(msr: u32, msb: u32, lsb: u32) -> WriteMsrRequest {
        Self { msr, msb, lsb }
    }

    /// Create a MSR write request from a raw u64 value
    #[inline]
    pub fn from_value(msr: u32, value: u64) -> WriteMsrRequest {
        let msb = ((value >> 32) & 0xFFFF_FFFF) as u32;
        let lsb = (value & 0xFFFF_FFFF) as u32;
        Self::new(msr, msb, lsb)
    }
}


impl GhcbProtocolRequest for WriteMsrRequest {
    type Response = ();

    fn execute_request(self, ghcb: &mut GhcbRequestExecutor) -> Self::Response {
        ghcb.raw().clear();

        ghcb.raw().set_field(GhcbU64Field::Rcx, self.msr as u64);
        ghcb.raw().set_field(GhcbU64Field::Rax, self.lsb as u64);
        ghcb.raw().set_field(GhcbU64Field::Rdx, self.msb as u64);

        ghcb.checked_vmgexit(GhcbExitCode::MsrProtocol, 1u64, 0);
    }
}

#[repr(transparent)]
pub struct ReadMsrRequest {
    msr: u32
}

impl GhcbProtocolRequest for ReadMsrRequest {
    /// (high, low) dwords returned by the request
    type Response = (u32, u32);

    fn execute_request(self, ghcb: &mut GhcbRequestExecutor) -> Self::Response {
        ghcb.raw().clear();
        ghcb.raw().set_field(GhcbU64Field::Rcx, self.msr as u64);
        ghcb.checked_vmgexit(GhcbExitCode::MsrProtocol, 0u64, 0);

        (
            ghcb.raw().get_field_if_valid(GhcbU64Field::Rdx).unwrap() as u32,
            ghcb.raw().get_field_if_valid(GhcbU64Field::Rax).unwrap() as u32,
        )
    }
}

impl ReadMsrRequest {
    #[inline]
    pub const fn new(msr: u32) -> Self {
        Self { msr }
    }

    #[inline]
    pub fn execute_request_to_u64(self, ghcb: &mut GhcbRequestExecutor) -> u64 {
        let (hi, lo) = self.execute_request(ghcb);
        (hi as u64) << 32 | (lo as u64)
    }

    #[inline]
    pub fn execute_to_u64<T: ChannelManager>(self) -> u64 {
        let (hi, lo) = self.execute::<T>();
        (hi as u64) << 32 | (lo as u64)
    }
}