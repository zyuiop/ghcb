use x86_64::structures::paging::{PageSize, PageTableFlags, PhysFrame};
use x86_64::VirtAddr;

/// A trait for types that can map frames to addresses
///
/// # Safety
///
///
pub unsafe trait MemoryMapper<S: PageSize> {
    /// Map a frame to a new address
    fn map_frame(&mut self, frame: PhysFrame<S>, flags: PageTableFlags) -> Option<VirtAddr>;
}
