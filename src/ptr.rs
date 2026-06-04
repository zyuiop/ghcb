use crate::mapping::PhysicalAllocator;
use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ops::{Deref, DerefMut};
use x86_64::{PhysAddr, VirtAddr};
use zerocopy::FromZeros;

pub struct OwnedPtrWithPhysAddr<T, Alloc: PhysicalAllocator + ?Sized> {
    phys_addr: PhysAddr,
    ptr: *mut T,

    _alloc: PhantomData<Alloc>,
}

impl<T, A: PhysicalAllocator + ?Sized> Debug for OwnedPtrWithPhysAddr<T, A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "OwnedPtrWithPhysAddr {{ ptr: {:p}, phys_addr: {:?} }}",
            self.ptr, self.phys_addr
        )
    }
}

impl<T, A: PhysicalAllocator + ?Sized> Deref for OwnedPtrWithPhysAddr<T, A> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl<T, A: PhysicalAllocator + ?Sized> DerefMut for OwnedPtrWithPhysAddr<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut().unwrap() }
    }
}

impl<T, A: PhysicalAllocator + ?Sized> OwnedPtrWithPhysAddr<T, A> {
    /// # Safety
    ///
    /// The returned pointer must be manually dropped
    pub unsafe fn leak(self) -> (*mut T, PhysAddr) {
        let me = ManuallyDrop::new(self);
        (me.ptr, me.phys_addr)
    }
}

impl<T, A: PhysicalAllocator + ?Sized> OwnedPtrWithPhysAddr<MaybeUninit<T>, A> {
    /// # Safety
    ///
    /// `virt` must be mapped to `phys`, and the allocation must be big enough to contain [T].
    ///
    /// The allocation must be zeroed to ensure [Self::to_init] works
    pub(crate) unsafe fn from_alloc(virt: VirtAddr, phys: PhysAddr) -> Self {
        Self {
            ptr: virt.as_mut_ptr(),
            phys_addr: phys,
            _alloc: PhantomData,
        }
    }
}

impl<T, A: PhysicalAllocator + ?Sized> Drop for OwnedPtrWithPhysAddr<T, A> {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: allocation comes from A, as ensured by the type bounds
            A::deallocate(self.virt_addr())
        }
    }
}

impl<T, A: PhysicalAllocator + ?Sized> OwnedPtrWithPhysAddr<MaybeUninit<T>, A> {
    pub unsafe fn assume_init(self) -> OwnedPtrWithPhysAddr<T, A> {
        // We don't want to drop the pointer!
        let me = ManuallyDrop::new(self);

        OwnedPtrWithPhysAddr::<T, A> {
            _alloc: PhantomData,
            ptr: me.ptr.cast::<T>(),
            phys_addr: me.phys_addr,
        }
    }
}

impl<T: FromZeros, A: PhysicalAllocator + ?Sized> OwnedPtrWithPhysAddr<MaybeUninit<T>, A> {
    pub fn to_init(self) -> OwnedPtrWithPhysAddr<T, A> {
        unsafe {
            // SAFETY: [FromZeros] guarantees it
            self.assume_init()
        }
    }
}

impl<T, A: PhysicalAllocator + ?Sized> OwnedPtrWithPhysAddr<T, A> {
    pub fn phys_addr(&self) -> PhysAddr {
        self.phys_addr
    }

    pub fn virt_addr(&self) -> VirtAddr {
        VirtAddr::new(self.ptr as u64)
    }
}

unsafe impl<T, A: PhysicalAllocator + ?Sized> Send for OwnedPtrWithPhysAddr<T, A> where
    T: Send + Sized
{
}

unsafe impl<T, A: PhysicalAllocator + ?Sized> Sync for OwnedPtrWithPhysAddr<T, A> where
    T: Sync + Sized
{
}
