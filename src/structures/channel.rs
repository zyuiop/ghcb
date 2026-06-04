use crate::mapping::PhysicalAllocator;
use crate::msr::GhcbMsr;
use crate::protocols::GhcbProtocolRequest;
use crate::structures::ghcb_page::GhcbPage;
#[cfg(feature = "multi-ghcb")]
use alloc::boxed::Box;
use core::fmt::Debug;
use core::ptr;
use core::sync::atomic::{AtomicU8, Ordering};
use x86_64::instructions::interrupts;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::structures::paging::{PageTableFlags, PhysFrame, Size4KiB};

pub struct GhcbChannel {
    /// The location in memory of the GHCB page
    frame_address: PhysFrame<Size4KiB>,

    /// A pointer to the GHCB page
    page: *mut GhcbPage,

    /// The number of currently used instances of this GHCB.
    /// Nested calls are possible because of interrupts, or because a function using the GHCB
    /// is itself making a call to [Self::with_ghcb].
    instance_count: AtomicU8,
}

unsafe impl Send for GhcbChannel {}

unsafe impl Sync for GhcbChannel {}

impl Debug for GhcbChannel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "GhcbChannel {{ frame_address: {:#x?}, page: {:p}, count: {}}}",
            self.frame_address,
            self.page,
            self.instance_count.load(Ordering::Relaxed)
        )
    }
}

const MAX_GHCB_DEPTH: u8 = cfg_select! {
    feature = "alloc" => 2,
    _ => 1
};

pub struct GhcbRequestExecutor<'a>(pub &'a mut GhcbPage);

impl GhcbRequestExecutor<'_> {
    #[inline(always)]
    pub fn raw(&mut self) -> &mut GhcbPage {
        &mut self.0
    }

    pub fn execute<T: GhcbProtocolRequest>(&mut self, request: T) -> T::Response {
        request.execute_request(self)
    }
}

impl GhcbChannel {
    /// The location in memory of the GHCB page
    pub fn phys_frame(&self) -> PhysFrame<Size4KiB> {
        self.frame_address
    }

    /// Initializes a GHCB channel for an already existing GHCB, which address is already set in
    /// the MSR.
    /// This is useful to re-use the EFI GHCB, for example.
    ///
    /// ## Safety
    ///
    /// This method assumes that the GHCB physical and virtual address are identical.
    /// This method should be called at most once, otherwise [Self::with_ghcb] safety guarantees
    /// are no longer valid
    pub unsafe fn identity_mapped() -> Option<Self> {
        let current_frame = GhcbMsr::get_current_ghcb_address()?;
        unsafe {
            let assumed_virt_addr =
                ptr::with_exposed_provenance_mut(current_frame.start_address().as_u64() as usize);
            Some(Self::new_registered(current_frame, assumed_virt_addr))
        }
    }

    /// Allocates a new GHCB channel and registers it.
    ///
    /// # Safety
    ///
    /// Any previous GHCB will become unregistered and must no longer be used (unless re-registered).
    pub unsafe fn allocate_register<A: PhysicalAllocator>(protocol_version: u16) -> Self {
        let info = GhcbMsr::get_info();
        if info.min_proto() > protocol_version || info.max_proto() < protocol_version {
            panic!(
                "GHCB version negotiation failed: wrong protocol version (expected: {protocol_version}, acceptable range: {}..{})",
                info.min_proto(),
                info.max_proto()
            );
        }

        let mut allocated = A::allocate_owned::<GhcbPage>(
            PageTableFlags::WRITABLE | PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE,
        )
        .expect("could not allocate frame for GHCB!")
        .to_init();
        allocated.set_protocol_version(protocol_version);

        let (ghcb_ptr, frame) = unsafe { allocated.leak() };
        let frame = PhysFrame::from_start_address(frame).unwrap();

        // Register the GHCB
        unsafe {
            GhcbMsr::register_and_set_ghcb(frame).expect("failed to register allocated GHCB");

            GhcbChannel::new_registered(frame, ghcb_ptr)
        }
    }

    /// Initializes a GHCB channel for an already registered GHCB, which address has already been
    /// set in the MSR.
    ///
    /// ## Safety
    ///
    /// This method assumes that the GHCB has been previously registered in AMD SEV-SNP.
    /// This method should be called at most once for a given pointer, otherwise [Self::with_ghcb]
    /// safety guarantees are no longer valid
    pub unsafe fn new_registered(
        frame_address: PhysFrame<Size4KiB>,
        pointer: *mut GhcbPage,
    ) -> Self {
        Self {
            frame_address,
            page: pointer,
            instance_count: AtomicU8::new(0),
        }
    }

    unsafe fn get_ghcb_ref(&self) -> GhcbRequestExecutor<'_> {
        let mut_ref = unsafe { self.page.as_mut().unwrap() };
        mut_ref.clear();
        mut_ref.set_phys_address(self.frame_address);
        GhcbRequestExecutor(mut_ref)
    }

    /// Gets the GHCB and clears it, ignoring any potentially set data inside
    ///
    /// ## Safety
    ///
    /// This will break any other GHCB usage. Applications using this should exit after use.
    pub unsafe fn with_ghcb_force<F>(&self, f: F)
    where
        F: FnOnce(GhcbRequestExecutor),
    {
        interrupts::disable();

        unsafe {
            f(self.get_ghcb_ref());
        }
    }

    pub fn with_ghcb<F, R>(&self, f: F) -> R
    where
        F: FnOnce(GhcbRequestExecutor) -> R,
    {
        without_interrupts(|| {
            unsafe {
                // Make sure the GHCB we're using is registered properly with the hypervisor
                // TODO: move to high level request execution?
                GhcbMsr::ensure_ghcb_address_is(self.frame_address);
            }

            let instance_id = self.instance_count.fetch_add(1, Ordering::AcqRel) + 1;
            if instance_id > MAX_GHCB_DEPTH {
                panic!("too many nested GHCB calls!")
            }

            // If this is not the first instance, first make a copy of the GHCB somewhere else in memory
            #[cfg(feature = "multi-ghcb")]
            let backup = if instance_id > 1 {
                Some(Box::new(unsafe { self.page.read_volatile() }))
            } else {
                None
            };

            #[cfg(not(feature = "multi-ghcb"))]
            assert_eq!(instance_id, 1);

            // Call the function
            // SAFETY: this is unsafe, but we have made sure to make a copy, and we will restore it or
            // panic before returning
            let ghcb = unsafe { self.get_ghcb_ref() };
            let result = f(ghcb);

            #[cfg(feature = "multi-ghcb")]
            if let Some(backup) = backup {
                unsafe {
                    self.page.write_volatile(*backup);
                }
            }

            let old_instance_id = self.instance_count.fetch_sub(1, Ordering::AcqRel);
            assert_eq!(instance_id, old_instance_id);

            result
        })
    }
}
