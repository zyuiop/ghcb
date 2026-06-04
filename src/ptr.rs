use core::fmt::{Debug, Formatter};
use core::ops::{Deref, DerefMut};
use x86_64::{PhysAddr, VirtAddr};

pub struct OwnedPtrWithPhysAddr<T> {
    phys_addr: PhysAddr,
    ptr: *mut T,
}

impl<T> Debug for OwnedPtrWithPhysAddr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "OwnedPtrWithPhysAddr {{ ptr: {:p}, phys_addr: {:?} }}", self.ptr, self.phys_addr)
    }
}

impl<T> Deref for OwnedPtrWithPhysAddr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl<T> DerefMut for OwnedPtrWithPhysAddr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut().unwrap() }
    }
}

impl<T: 'static> OwnedPtrWithPhysAddr<T> {
    /// Create an owned pointer from the provided static mutable reference.
    pub fn new(ptr: &'static mut T, address: PhysAddr) -> Self {
        Self {
            ptr: ptr as *mut T, phys_addr: address,
        }
    }
}

impl<T: 'static> OwnedPtrWithPhysAddr<T> {
    pub fn phys_addr(&self) -> PhysAddr {
        self.phys_addr
    }

    pub fn virt_addr(&self) -> VirtAddr {
        VirtAddr::new(self.ptr as u64)
    }
}

unsafe impl<T> Send for OwnedPtrWithPhysAddr<T> where T: Send + Sized {}

unsafe impl<T> Sync for OwnedPtrWithPhysAddr<T> where T: Sync + Sized {}
