use x86_64::structures::paging::{PageSize, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::VirtAddr;
use crate::msr::GhcbMsr;
use crate::msr::page_state_change::{PageStateChangeRequest, PageStateOperation};

/// A trait for types that can map frames to addresses
///
/// # Safety
///
///
pub unsafe trait MemoryMapper<S: PageSize> {
    /// Map a frame to a new address
    fn map_frame(&mut self, frame: PhysFrame<S>, flags: PageTableFlags) -> Option<VirtAddr>;
}

pub trait SharedMemoryMapper<S: PageSize> {
    fn map_frame_make_shared(&mut self, frame: PhysFrame<S>, flags: PageTableFlags) -> Option<VirtAddr>;
}

impl<MM: MemoryMapper<Size4KiB>> SharedMemoryMapper<Size4KiB> for MM {
    fn map_frame_make_shared(&mut self, frame: PhysFrame<Size4KiB>, flags: PageTableFlags) -> Option<VirtAddr> {
        assert!(!flags.is_encrypted(), "cannot make a shared page encrypted");

        let va = self.map_frame(frame, flags)?;

        // Invalidate entry
        // pvalidate(PageStateChangePageSize::PageSize4KB, false, va);
        // Does not appear to work with non identity mapped addresses - which is weird
        // In any case, the next VM Exit will cause the hypervisor to remap the page, which should invalidate it

        // Tell the hypervisor to make it shared
        let result = unsafe {
            GhcbMsr::execute(
                PageStateChangeRequest::create(frame, PageStateOperation::AssignShared)
            )
        };

        if result.is_successful() {
            Some(va)
        } else {
            None
        }
    }
}
