use crate::msr::page_state_change::{PageStateChangeRequest, PageStateOperation};
use crate::msr::GhcbMsr;
use crate::ptr::OwnedPtrWithPhysAddr;
use core::mem::MaybeUninit;
use x86_64::structures::paging::{PageSize, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::VirtAddr;

/// # Safety
///
/// Similar to [core::alloc::Allocator]
pub unsafe trait PhysicalAllocator {
    /// Allocates a 4KiB frame and maps it with the provided page flags.
    fn allocate(flags: PageTableFlags) -> Option<(PhysFrame<Size4KiB>, VirtAddr)>;

    /// # Safety
    ///
    /// The memory must have been allocated by this allocator
    unsafe fn deallocate(addr: VirtAddr);

    /// Allocates a new frame and page, zeroes it, and wraps it in an OwnedPtr
    fn allocate_owned<T>(flags: PageTableFlags) -> Option<OwnedPtrWithPhysAddr<MaybeUninit<T>, Self>> {
        assert!(size_of::<T>() <= Size4KiB::SIZE as usize, "object too big to allocate");

        let (frame, virt) = Self::allocate(flags)?;

        unsafe {
            // zeroise allocated memory
            virt.as_mut_ptr::<T>().write_bytes(0, 1);
        }

        if !flags.is_encrypted() {
            // Tell the hypervisor to make the frame shared
            let result = unsafe {
                GhcbMsr::execute(
                    PageStateChangeRequest::create(frame, PageStateOperation::AssignShared)
                )
            };

            if !result.is_successful() {
                panic!("failed to share page with hypervisor: {}", result.0.unwrap())
            }
        }


        Some(unsafe {
            // SAFETY: we have verified the
            OwnedPtrWithPhysAddr::<_, Self>::from_alloc(virt, frame.start_address())
        })
    }
}