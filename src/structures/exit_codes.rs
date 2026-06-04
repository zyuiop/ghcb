use zerocopy_derive::FromZeros;

#[derive(Debug, Clone, Copy, FromZeros)]
#[repr(u64)]
pub enum GhcbExitCode {
    None = 0x00,

    DR7Read = 0x27,
    DR7Write = 0x37,

    RdTsc = 0x6e,
    RdPmc = 0x6f,

    CPUID = 0x72,

    Invd = 0x76,

    IoIoProtocol = 0x7b,
    MsrProtocol = 0x7c,
    VmmCall = 0x81,
    RdTscp = 0x87,
    WbInvd = 0x89,

    Monitor = 0x8a,
    MWait = 0x8b,

    MmioRead = 0x8000_0001,
    MmioWrite = 0x8000_0002,
    NmiComplete = 0x8000_0003,
    ApResetHold = 0x8000_0004,
    ApJumpTable = 0x8000_0005,
    PageStateChange = 0x8000_0010,

    SnpGuestRequest = 0x8000_0011,
    SnpGuestExtendedRequest = 0x8000_0012,
    SnpApCreation = 0x8000_0013,

    HvDoorbellPages = 0x8000_0014,
    HvIpi = 0x8000_0015,
    HvTimer = 0x8000_0016,

    ApicIdList = 0x8000_0017,

    SnpRunVmpl = 0x8000_0018,
    SnpTioGuestRequest = 0x8000_0019,
    SecureAvic = 0x8000_001a,

    HypervisorFeatureSupport = 0x8000_fffd,

    TerminationRequest = 0x8000_fffe,
    UnsupportedEvent = 0x8000_ffff,
}
