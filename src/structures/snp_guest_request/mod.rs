pub mod error;
pub mod attest;
pub mod shared_page;

use aes_gcm::{AeadInOut, KeyInit};
use bitfield_struct::bitenum;
use zerocopy::{FromBytes, IntoBytes};
use crate::structures::snp_secrets_page::SecretsPageAccessor;


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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone)]
#[repr(u32)]
#[bitenum]
pub enum GuestProtocolStatusCode {
    Success = 0,

    /// The platform state is invalid for this command
    InvalidPlatformState = 0x1,

    /// The guest state is invalid for this command
    InvalidGuestState = 0x2,

    /// The platform configuration is invalid
    InvalidConfig = 0x0003,

    /// A memory buffer is too small.
    InvalidLength = 0x0004,

    /// The platform is already owned
    AlreadyOwned = 0x0005,

    /// The certificate is invalid
    InvalidCertificate = 0x0006,

    /// Request is not allowed by guest policy
    PolicyFailure = 0x0007,

    /// The guest is inactive
    Inactive = 0x0008,

    /// The address provided is invalid
    InvalidAddress = 0x0009,

    /// The provided signature is invalid
    BadSignature = 0x000A,

    /// The provided measurement is invalid
    BadMeasurement = 0x000B,

    /// The ASID is already owned
    AsidOwned = 0x000C,

    /// The ASID is invalid
    InvalidAsid = 0x000D,

    /// WBINVD instruction required
    WbinvdRequired = 0x000E,

    /// DF_FLUSH invocation required
    DfFlushRequired = 0x000F,

    /// The guest handle is invalid
    InvalidGuest = 0x0010,

    /// The command issued is invalid.
    InvalidCommand = 0x0011,

    /// The guest is active.
    Active = 0x0012,

    /// A hardware condition has occurred affecting the
    /// platform. It is safe to re-allocate parameter
    /// buffers.
    HwerrorPlatform = 0x0013,

    /// A hardware condition has occurred affecting the
    /// platform. Re-allocating parameter buffers is not
    /// safe.
    HwerrorUnsafe = 0x0014,

    /// Feature is unsupported.
    Unsupported = 0x0015,

    /// A parameter is invalid.
    InvalidParam = 0x0016,

    /// The SEV FW has run out of a resource
    /// necessary to complete the command.
    ResourceLimit = 0x0017,

    /// The part-specific SEV data failed integrity checks.
    SecureDataInvalid = 0x0018,

    /// A Mailbox mode command was sent while the
    /// SEV FW was in Ring Buffer mode. Ring Buffer
    /// mode has been exited; the Mailbox mode
    /// command has been ignored. Retry is
    /// recommended.
    RbModeExited = 0x001F,

    /// The RMP page size is incorrect.
    InvalidPageSize = 0x0019,

    /// The RMP page state is incorrect.
    InvalidPageState = 0x001A,

    /// The metadata entry is invalid.
    InvalidMdataEntry = 0x001B,

    /// The page ownership is incorrect.
    InvalidPageOwner = 0x001C,

    /// The AEAD algorithm would have overflowed.
    AeadOflow = 0x001D,

    /// The RMP must be reinitialized.
    RmpInitRequired = 0x0020,

    /// SVN of provided image is lower than the committed SVN.
    BadSvn = 0x0021,

    /// Firmware version anti-rollback
    BadVersion = 0x0022,

    /// An invocation of SNP_SHUTDOWN is required tocomplete this action.
    ShutdownRequired = 0x0023,

    /// Update of the firmware internal state or a guest contextpage has failed.
    UpdateFailed = 0x0024,

    /// Installation of the committed firmware image required.
    RestoreRequired = 0x0025,

    /// The RMP initialization failed.
    RmpInitFailed = 0x0026,

    /// The key requested is invalid, not present, or not allowed.
    InvalidKey = 0x0027,

    #[fallback]
    InvalidError = 0xFFFF
}

pub trait SNPGuestRequest: Sized + FromBytes + IntoBytes {
    type ResponseType: SNPGuestResponse;

    fn message_type() -> MessageType;
}

pub trait SNPGuestResponse: Sized + FromBytes + IntoBytes {
    fn message_type() -> MessageType;
}