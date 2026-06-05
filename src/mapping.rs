use crate::ptr::OwnedPtrWithPhysAddr;
use core::mem::MaybeUninit;
use x86_64::VirtAddr;
use x86_64::structures::paging::{PageSize, PageTableFlags, PhysFrame, Size4KiB};

pub mod mapping_utils {
    use x86_64::structures::paging::{PhysFrame, Size2MiB, Size4KiB};
    use x86_64::VirtAddr;
    use crate::instructions::pvalidate::pvalidate;
    use crate::msr::GhcbMsr;
    use crate::msr::page_state_change::{PageStateChangeRequest, PageStateOperation};
    use crate::protocols::change_page_state::{ChangePageStateRequest, PageStateChangeEntry, PageStateChangeOperation, PageStateChangePageSize};
    use crate::protocols::GhcbProtocolRequest;
    use crate::structures::ChannelManager;

    /// Makes a frame shared with the hypervisor.
    ///
    /// # Safety
    ///
    /// virt_addr must map to the provided frame
    pub unsafe fn make_shared(frame: PhysFrame<Size4KiB>, virt_addr: VirtAddr) {
        // Rescind RMP validation for page
        pvalidate(PageStateChangePageSize::PageSize4KB, false, virt_addr);

        // Tell the hypervisor to make the frame shared
        let result = unsafe {
            GhcbMsr::execute(PageStateChangeRequest::create(
                frame,
                PageStateOperation::AssignShared,
            ))
        };

        if !result.is_successful() {
            panic!(
                "failed to share page with hypervisor: {}",
                result.0.unwrap()
            )
        }
    }

    /// Makes a frame non shared with the hypervisor.
    ///
    /// # Safety
    ///
    /// virt_addr must map to the provided frame
    pub unsafe fn make_private(frame: PhysFrame<Size4KiB>, virt_addr: VirtAddr) {
        // Tell the hypervisor to make the frame private
        let result = unsafe {
            GhcbMsr::execute(PageStateChangeRequest::create(
                frame,
                PageStateOperation::AssignPrivate,
            ))
        };

        if !result.is_successful() {
            panic!(
                "failed to share page with hypervisor: {}",
                result.0.unwrap()
            )
        }

        // Grant validation for page
        pvalidate(PageStateChangePageSize::PageSize4KB, true, virt_addr);
    }

    /// Makes a frame shared with the hypervisor.
    ///
    /// # Safety
    ///
    /// virt_addr must map to the provided frame
    pub unsafe fn make_shared_large<C: ChannelManager>(frame: PhysFrame<Size2MiB>, virt_addr: VirtAddr) {
        // Rescind RMP validation for page
        pvalidate(PageStateChangePageSize::PageSize2MB, false, virt_addr);

        // Tell the hypervisor to make the frame shared
        ChangePageStateRequest::new(&[
            PageStateChangeEntry::new_for_frame(frame, PageStateChangeOperation::PageAssignShared)
        ]).execute::<C>().expect("failed to share page with hypervisor");
    }

    /// Makes a frame non shared with the hypervisor.
    ///
    /// # Safety
    ///
    /// virt_addr must map to the provided frame
    pub unsafe fn make_private_large<C: ChannelManager>(frame: PhysFrame<Size4KiB>, virt_addr: VirtAddr) {
        // Tell the hypervisor to make the frame private
        ChangePageStateRequest::new(&[
            PageStateChangeEntry::new_for_frame(frame, PageStateChangeOperation::PageAssignPrivate)
        ]).execute::<C>().expect("failed to unshare page with hypervisor");

        // Grant validation for page
        pvalidate(PageStateChangePageSize::PageSize2MB, true, virt_addr);
    }
}

/// # Safety
///
/// Similar to [core::alloc::Allocator]
pub unsafe trait PhysicalAllocator {
    /// Allocates a 4KiB frame and maps it with the provided page flags.
    ///
    /// # Safety
    ///
    /// If [PageTableFlags::is_encrypted] is false, the allocated frame must have been shared with
    /// the hypervisor. Implementors can use [mapping_utils::make_shared] for that.
    ///
    /// Allocators must zero the memory before returning it.
    fn allocate_zeroed(flags: PageTableFlags) -> Option<(PhysFrame<Size4KiB>, VirtAddr)>;

    /// Deallocates the 4KB frame starting at given address.
    ///
    /// # Safety
    ///
    /// The memory must have been allocated by this allocator.
    ///
    /// If implementors return the frame to a usable pool, they must ensure it is set to private
    /// again. They can use [mapping_utils::make_private] for that.
    unsafe fn deallocate(addr: VirtAddr);

    /// Allocates a new frame and page, zeroes it, and wraps it in an OwnedPtr
    fn allocate_owned<T>(
        flags: PageTableFlags,
    ) -> Option<OwnedPtrWithPhysAddr<MaybeUninit<T>, Self>> {
        assert!(
            size_of::<T>() <= Size4KiB::SIZE as usize,
            "object too big to allocate"
        );

        let (frame, virt) = Self::allocate_zeroed(flags)?;
        Some(unsafe {
            // SAFETY: memory is zeroed
            OwnedPtrWithPhysAddr::<_, Self>::from_alloc(virt, frame.start_address())
        })
    }
}
