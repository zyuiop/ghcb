use core::error::Error;
use core::fmt::{Display, Formatter, Write};

#[derive(Debug)]
#[repr(u64)]
pub enum MalformedGhcbError {
    GhcbNotRegistered = 0x0001,
    InvalidGhcbUsageValue = 0x0002,
    InvalidScratch = 0x0003,
    MissingRequiredFields = 0x0004,
    InvalidEventInput = 0x0005,
    InvalidEvent = 0x0006,
    Reserved(u64),
    HypervisorSpecific(u64),
}

impl Display for MalformedGhcbError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            MalformedGhcbError::GhcbNotRegistered => {
                f.write_str("GHCB was not registered (AMD SEV-SNP)")
            }
            MalformedGhcbError::InvalidGhcbUsageValue => f.write_str("GHCB usage field is invalid"),
            MalformedGhcbError::InvalidScratch => {
                f.write_str("GHCB scratch address is invalid or cannot be mapped")
            }
            MalformedGhcbError::MissingRequiredFields => {
                f.write_str("Fields required for call were missing in GHCB")
            }
            MalformedGhcbError::InvalidEventInput => {
                f.write_str("NAE event input (ExitInfo values) is invalid")
            }
            MalformedGhcbError::InvalidEvent => f.write_str("NAE exit code is invalid"),
            MalformedGhcbError::Reserved(v) => {
                write!(f, "Error code reserved for future use: {v:x}")
            }
            MalformedGhcbError::HypervisorSpecific(v) => {
                write!(f, "Hypervisor specific error code: {v:x}")
            }
        }
    }
}

impl Error for MalformedGhcbError {}

impl From<u64> for MalformedGhcbError {
    fn from(value: u64) -> Self {
        match value {
            1 => Self::GhcbNotRegistered,
            2 => Self::InvalidGhcbUsageValue,
            3 => Self::InvalidScratch,
            4 => Self::MissingRequiredFields,
            5 => Self::InvalidEventInput,
            6 => Self::InvalidEvent,
            other if other <= 0xffff => Self::Reserved(other),
            other => Self::HypervisorSpecific(other),
        }
    }
}
