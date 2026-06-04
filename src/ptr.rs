use core::fmt::{Debug, Formatter};
use core::ops::{Deref, DerefMut};
use core::ptr;
use x86_64::{PhysAddr, VirtAddr};

pub struct OwnedPtrWithPhysAddr<T: 'static> {
    ptr: &'static mut T,
    phys_addr: PhysAddr,
}

impl<T: 'static> Debug for OwnedPtrWithPhysAddr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "OwnedPtrWithPhysAddr {{ ptr: {:p}, phys_addr: {:?} }}", self.ptr, self.phys_addr)
    }
}

impl<T: 'static> Deref for OwnedPtrWithPhysAddr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

impl<T: 'static> DerefMut for OwnedPtrWithPhysAddr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ptr
    }
}

impl<T: 'static> OwnedPtrWithPhysAddr<T> {
    /// Create an owned pointer from the provided static mutable reference.
    pub fn new(ptr: &'static mut T, address: PhysAddr) -> Self {
        Self {
            ptr, phys_addr: address,
        }
    }

    pub fn phys_addr(&self) -> PhysAddr {
        self.phys_addr
    }

    pub fn virt_addr(&self) -> VirtAddr {
        VirtAddr::new(ptr::from_ref(self.ptr).addr() as u64)
    }
}

unsafe impl<T: 'static> Send for OwnedPtrWithPhysAddr<T> where T: Send + Sized {}

unsafe impl<T: 'static> Sync for OwnedPtrWithPhysAddr<T> where T: Sync + Sized {}
