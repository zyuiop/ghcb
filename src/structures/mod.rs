use x86_64::structures::paging::{PhysFrame, Size4KiB};
use x86_64::VirtAddr;
use crate::structures::channel::GhcbChannel;

pub mod ghcb_page;
pub mod exit_codes;
pub mod channel;
pub mod errors;
pub mod vmsa;

pub trait ChannelManager {
    /// Gets the global channel for the current "context".
    fn get_channel() -> &'static GhcbChannel;
}