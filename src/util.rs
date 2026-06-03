use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use x86_64::{PhysAddr, VirtAddr};

pub struct OwnedPtr<T> {
    ptr: NonNull<T>,
}

impl<T> OwnedPtr<T> {
    /// Create an owned pointer from the provided pointer.
    ///
    /// ## Safety
    ///
    /// The pointer used to create this OwnedPtr must be unique and valid
    pub unsafe fn new(ptr: NonNull<T>) -> OwnedPtr<T> {
        OwnedPtr { ptr }
    }

    pub fn with_phys_addr(self, phys_addr: PhysAddr) -> OwnedPtrWithPhysAddr<T> {
        OwnedPtrWithPhysAddr {
            ptr: self,
            phys_addr
        }
    }

    pub fn virt_addr(&self) -> VirtAddr {
        VirtAddr::new(self.ptr.addr().get() as u64)
    }
}

impl<T> Deref for OwnedPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            // SAFETY: by borrow checker (caller has a reference to us)
            self.ptr.as_ref()
        }
    }
}

impl<T> DerefMut for OwnedPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            // SAFETY: by borrow checker (caller has a mutable reference to us)
            self.ptr.as_mut()
        }
    }
}

pub struct OwnedPtrWithPhysAddr<T> {
    ptr: OwnedPtr<T>,
    phys_addr: PhysAddr
}

impl<T> Deref for OwnedPtrWithPhysAddr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.ptr.deref()
    }
}

impl<T> DerefMut for OwnedPtrWithPhysAddr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ptr.deref_mut()
    }
}

impl<T> OwnedPtrWithPhysAddr<T> {
    /// Create an owned pointer from the provided pointer.
    ///
    /// ## Safety
    ///
    /// The pointer used to create this OwnedPtr must be unique and valid
    pub unsafe fn new(ptr: NonNull<T>, address: PhysAddr) -> Self {
        OwnedPtr::new(ptr).with_phys_addr(address)
    }

    pub fn phys_addr(&self) -> PhysAddr {
        self.phys_addr
    }

    pub fn forget_phys_addr(self) -> OwnedPtr<T> {
        self.ptr
    }

    pub fn virt_addr(&self) -> VirtAddr {
        self.ptr.virt_addr()
    }
}

unsafe impl<T> Send for OwnedPtrWithPhysAddr<T>
where T: Send + Sized {}

unsafe impl<T> Sync for OwnedPtrWithPhysAddr<T>
where T: Sync + Sized {}