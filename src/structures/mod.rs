use crate::structures::channel::GhcbChannel;

pub mod ghcb_page;
pub mod exit_codes;
pub mod channel;
pub mod errors;
pub mod vmsa;
#[cfg(feature = "snp")]
pub mod snp_guest_request;
#[cfg(feature = "snp")]
pub mod snp_secrets_page;
#[cfg(feature = "snp")]
pub mod snp_cpuid_page;

pub trait ChannelManager {
    /// Gets the global channel for the current "context".
    fn get_channel() -> &'static GhcbChannel;
}