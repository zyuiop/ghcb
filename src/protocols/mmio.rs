use crate::protocols::GhcbProtocolRequest;
use crate::structures::ChannelManager;
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::exit_codes::GhcbExitCode;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ops::Add;
use core::{ptr, slice};
use x86_64::PhysAddr;

pub struct MmioRead<'a> {
    source: PhysAddr,
    target: &'a mut [u8],
}

pub struct MmioWrite<'a> {
    source: &'a [u8],
    target: PhysAddr,
}

pub struct MmioPtr<T, C: ChannelManager> {
    addr: PhysAddr,
    _channel: PhantomData<C>,
    _typ: PhantomData<T>,
}

impl<T, C: ChannelManager> MmioPtr<T, C> {
    pub fn new(addr: PhysAddr) -> Self {
        Self {
            addr,
            _channel: PhantomData,
            _typ: PhantomData,
        }
    }

    /// Read an object from an MMIO interface, using a GHCB call
    ///
    /// ## Safety
    ///
    /// The source address must be valid and contain an object that can be transmuted to type [T]
    pub unsafe fn read_volatile(&self) -> T {
        let mut output = MaybeUninit::<T>::uninit();
        let output_ptr = output.as_mut_ptr().cast::<u8>();
        unsafe {
            let output_slice = slice::from_raw_parts_mut(output_ptr, size_of::<T>());
            MmioRead::new(self.addr, output_slice).execute::<C>();
            output.assume_init()
        }
    }

    /// Write an object to an MMIO interface, using a GHCB call
    ///
    /// ## Safety
    ///
    /// The target address must be valid and big enough to contain an object of type [T]
    ///
    pub unsafe fn write_volatile(&self, source: T) {
        let ptr = ptr::from_ref(&source).cast::<u8>();
        unsafe {
            let slice = slice::from_raw_parts(ptr, size_of::<T>());
            MmioWrite::new(slice, self.addr).execute::<C>();
        }
    }
}

impl<'a> MmioRead<'a> {
    /// ## Safety
    ///
    /// The source address and `target.len()` bytes must be valid
    pub unsafe fn new(source_addr: PhysAddr, target: &'a mut [u8]) -> Self {
        Self {
            source: source_addr,
            target,
        }
    }
}

impl<'a> MmioWrite<'a> {
    /// ## Safety
    ///
    /// The target address and `source.len()` bytes must be valid
    pub unsafe fn new(source: &'a [u8], target_addr: PhysAddr) -> Self {
        Self {
            source,
            target: target_addr,
        }
    }
}

impl GhcbProtocolRequest for MmioWrite<'_> {
    type Response = ();

    fn execute_request(self, ghcb: &mut GhcbRequestExecutor) -> Self::Response {
        let buff_size = ghcb.raw().shared_buffer_size();

        // If the data is too big for a single MMIO instruction, call recursively
        if self.source.len() > buff_size {
            for offset in (0..self.source.len()).step_by(buff_size) {
                let end = if offset + buff_size > self.source.len() {
                    self.source.len()
                } else {
                    offset + buff_size
                };

                MmioWrite {
                    target: self.target.add(offset as u64),
                    source: &self.source[offset..end],
                }
                .execute_request(ghcb)
            }
            return ();
        }

        // Copy the data to the buffer
        ghcb.raw().use_shared_buffer();
        ghcb.raw().copy_to_shared_buffer(self.source);

        // Issue the call
        ghcb.checked_vmgexit(
            GhcbExitCode::MmioWrite,
            self.target.as_u64(),
            self.source.len() as u64,
        )
    }
}

impl GhcbProtocolRequest for MmioRead<'_> {
    type Response = ();

    fn execute_request(self, ghcb: &mut GhcbRequestExecutor) -> Self::Response {
        let buff_size = ghcb.raw().shared_buffer_size();

        // If the data is too big for a single MMIO instruction, call recursively
        if self.target.len() > buff_size {
            for offset in (0..self.target.len()).step_by(buff_size) {
                let end = if offset + buff_size > self.target.len() {
                    self.target.len()
                } else {
                    offset + buff_size
                };

                MmioRead {
                    source: self.source.add(offset as u64),
                    target: &mut self.target[offset..end],
                }
                .execute_request(ghcb)
            }
            return ();
        }

        // Issue the call
        ghcb.raw().use_shared_buffer();
        ghcb.checked_vmgexit(
            GhcbExitCode::MmioRead,
            self.source.as_u64(),
            self.target.len() as u64,
        );

        // Copy data from the buffer
        ghcb.raw().copy_from_shared_buffer(self.target);
    }
}
