use core::mem::offset_of;
use core::ops::Add;
use core::ptr;
use volatile::access::ReadOnly;
use volatile::VolatileRef;
use x86_64::PhysAddr;
use x86_64::structures::paging::{PhysFrame, Size4KiB};
use crate::structures::exit_codes::GhcbExitCode;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GhcbSaveArea {
    _reserved_0x0: [u8; 203],
    cpl: u8,
    _reserved_0xcc: [u8; 116],
    xss: u64,
    _reserved_0x148: [u8; 24],
    dr7: u64,
    _reserved_0x168: [u8; 16],
    rip: u64,
    _reserved_0x180: [u8; 88],
    rsp: u64,
    _reserved_0x1e0: [u8; 24],
    rax: u64,
    _reserved_0x200: [u8; 264],
    rcx: u64,
    rdx: u64,
    rbx: u64,
    _reserved_0x320: [u8; 8],
    rbp: u64,
    rsi: u64,
    rdi: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    _reserved_0x380: [u8; 16],
    sw_exit_code: GhcbExitCode,
    sw_exit_info_1: u64,
    sw_exit_info_2: u64,
    sw_scratch: u64,
    _reserved_0x3b0: [u8; 56],
    xcr0: u64,
    valid_bitmap: [u8; 16],
    x87_state_gpa: u64,
}

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
enum GhcbOtherField {
    ExitCode = offset_of!(GhcbSaveArea, sw_exit_code),
    ScratchAddress = offset_of!(GhcbSaveArea, sw_scratch)
}

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum GhcbU64Field {
    Cpl = offset_of!(GhcbSaveArea, cpl),
    Xss = offset_of!(GhcbSaveArea, xss),
    Dr7 = offset_of!(GhcbSaveArea, dr7),
    Rip = offset_of!(GhcbSaveArea, rip),
    Rsp = offset_of!(GhcbSaveArea, rsp),
    Rax = offset_of!(GhcbSaveArea, rax),
    Rcx = offset_of!(GhcbSaveArea, rcx),
    Rdx = offset_of!(GhcbSaveArea, rdx),
    Rbx = offset_of!(GhcbSaveArea, rbx),
    Rbp = offset_of!(GhcbSaveArea, rbp),
    Rsi = offset_of!(GhcbSaveArea, rsi),
    Rdi = offset_of!(GhcbSaveArea, rdi),
    R8 = offset_of!(GhcbSaveArea, r8),
    R9 = offset_of!(GhcbSaveArea, r9),
    R10 = offset_of!(GhcbSaveArea, r10),
    R11 = offset_of!(GhcbSaveArea, r11),
    R12 = offset_of!(GhcbSaveArea, r12),
    R13 = offset_of!(GhcbSaveArea, r13),
    R14 = offset_of!(GhcbSaveArea, r14),
    R15 = offset_of!(GhcbSaveArea, r15),
    SwExitInfo1 = offset_of!(GhcbSaveArea, sw_exit_info_1),
    SwExitInfo2 = offset_of!(GhcbSaveArea, sw_exit_info_2),
    XCr0 = offset_of!(GhcbSaveArea, xcr0),
}

impl GhcbSaveArea {
    fn offset_mut_ptr<T>(&mut self, offset: usize) -> *mut T {
        assert!((offset + size_of::<T>()) <= size_of::<GhcbSaveArea>());
        let addr = ptr::from_mut(self).addr() + offset;
        addr as *mut T
    }

    fn offset_ptr<T>(&self, offset: usize) -> *const T {
        let addr = ptr::from_ref(self).addr() + offset;
        addr as *const T
    }

    fn set_field_valid(&mut self, offset: usize) {
        let offset = offset >> 3;
        assert!(offset < (size_of_val(&self.valid_bitmap) * 8));

        let validity_ptr: *mut u8 = self.offset_mut_ptr(offset_of!(GhcbSaveArea, valid_bitmap));
        unsafe {
            let validity_ptr = validity_ptr.add(offset / 8);
            let valid = validity_ptr.read_volatile();
            validity_ptr.write_volatile(valid | (1 << (offset % 8)));
        }
    }

    fn is_field_valid(&self, offset: usize) -> bool {
        let offset = offset >> 3;
        assert!(offset < (size_of_val(&self.valid_bitmap) * 8));

        let validity_ptr: *const u8 = self.offset_ptr(offset_of!(GhcbSaveArea, valid_bitmap));
        unsafe {
            let validity_ptr = validity_ptr.add(offset / 8);
            let valid = validity_ptr.read_volatile();

            valid & 1 << (offset % 8) as u8 != 0
        }
    }
}

impl Default for GhcbSaveArea {
    fn default() -> Self {
        Self {
            _reserved_0x0: [0; 203],
            cpl: 0,
            _reserved_0xcc: [0; 116],
            xss: 0,
            _reserved_0x148: [0; 24],
            dr7: 0,
            _reserved_0x168: [0; 16],
            rip: 0,
            _reserved_0x180: [0; 88],
            rsp: 0,
            _reserved_0x1e0: [0; 24],
            rax: 0,
            _reserved_0x200: [0; 264],
            rcx: 0,
            rdx: 0,
            rbx: 0,
            _reserved_0x320: [0; 8],
            rbp: 0,
            rsi: 0,
            rdi: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            _reserved_0x380: [0; 16],
            sw_exit_code: GhcbExitCode::UnsupportedEvent,
            sw_exit_info_1: 0,
            sw_exit_info_2: 0,
            sw_scratch: 0,
            _reserved_0x3b0: [0; 56],
            xcr0: 0,
            valid_bitmap: [0; 16],
            x87_state_gpa: 0,
        }
    }
}

const GHCB_SHARED_BUF_SIZE: usize =	2032;


#[repr(C, align(0x1000))]
#[derive(Debug, Clone)]
pub struct GhcbPage {
    save: GhcbSaveArea,

    reserved_save: [u8; 2048 - size_of::<GhcbSaveArea>()],

    shared_buffer: [u8; GHCB_SHARED_BUF_SIZE],

    reserved_0xff0: [u8; 10],
    protocol_version: u16,	/* negotiated SEV-ES/GHCB protocol version */
    ghcb_usage: u32,
}

impl Default for GhcbPage {
    fn default() -> Self {
        Self {
            save: GhcbSaveArea::default(),
            reserved_save: [0; 2048 - size_of::<GhcbSaveArea>()],
            shared_buffer: [0; GHCB_SHARED_BUF_SIZE],
            reserved_0xff0: [0; 10],
            protocol_version: 0,
            ghcb_usage: 0,
        }
    }
}
pub(crate) const GHCB_SCRATCH_OFFSET: u64 = core::mem::offset_of!(GhcbPage, shared_buffer) as u64;

impl GhcbPage {
    pub const fn shared_buffer_size(&self) -> usize {
        size_of_val(&self.shared_buffer)
    }


    pub fn clear(&mut self) {
        unsafe {
            // Preserve scratch address (set by the manager)
            let scratch = ptr::from_mut(&mut self.save.sw_scratch).read_volatile();

            ptr::from_mut(&mut self.save).write_volatile(GhcbSaveArea::default());
            ptr::from_mut(&mut self.shared_buffer).write_volatile([0; GHCB_SHARED_BUF_SIZE]);

            // Write back the scratch addres
            ptr::from_mut(&mut self.save.sw_scratch).write_volatile(scratch);
        }
    }

    pub fn shared_buffer_mut(&mut self) -> VolatileRef<'_, [u8; GHCB_SHARED_BUF_SIZE]> {
        VolatileRef::from_mut_ref(&mut self.shared_buffer)
    }

    pub fn shared_buffer(&self) -> VolatileRef<'_, [u8; GHCB_SHARED_BUF_SIZE], ReadOnly> {
        VolatileRef::from_ref(&self.shared_buffer)
    }

    /// ## Safety
    ///
    /// Reads from the returned pointer must be volatile
    pub unsafe fn shared_buffer_raw(&self) -> *const [u8; GHCB_SHARED_BUF_SIZE] {
        ptr::from_ref(&self.shared_buffer)
    }

    pub unsafe fn copy_to_shared_buffer_raw(&mut self, src: *const u8, size: usize) {
        assert!(size <= self.shared_buffer_size());

        let mut src_u64 = src as *const u64;
        let mut shared_u64 = ptr::from_mut(&mut self.shared_buffer) as *mut u64;

        let mut size = size;
        while size >= 8 {
            unsafe {
                shared_u64.write_volatile(src_u64.read());
                src_u64 = src_u64.add(1);
                shared_u64 = shared_u64.add(1);
                size -= 8;
            }
        }

        let mut src_u8 = src_u64 as *const u8;
        let mut shared_u8 = shared_u64 as *mut u8;
        while size > 0 {
            unsafe {
                shared_u8.write_volatile(src_u8.read());
                src_u8 = src_u8.add(1);
                shared_u8 = shared_u8.add(1);
                size -= 1;
            }
        }
    }

    pub unsafe fn copy_from_shared_buffer_raw(&self, target: *mut u8, size: usize) {
        assert!(size <= self.shared_buffer_size());


        let mut src_u64 = ptr::from_ref(&self.shared_buffer) as *const u64;
        let mut target_u64 = target as *mut u64;

        let mut size = size;
        while size >= 8 {
            unsafe {
                target_u64.write(src_u64.read_volatile());
                src_u64 = src_u64.add(1);
                target_u64 = target_u64.add(1);
                size -= 8;
            }
        }

        let mut src_u8 = src_u64 as *const u8;
        let mut target_u8 = target_u64 as *mut u8;
        while size > 0 {
            unsafe {
                target_u8.write(src_u8.read_volatile());
                src_u8 = src_u8.add(1);
                target_u8 = target_u8.add(1);
                size -= 1;
            }
        }
    }

    /// Copies a slice to the shared buffer.
    /// This does not mark the shared buffer as used.
    ///
    /// ## Panics
    ///
    /// If the size of the buffer is bigger than the shared buffer
    pub fn copy_to_shared_buffer(&mut self, buffer: &[u8]) {
        unsafe {
            self.copy_to_shared_buffer_raw(buffer.as_ptr(), buffer.len());
        }
    }

    /// Copies from the shared buffer to a slice.
    /// This does not mark the shared buffer as used.
    ///
    /// ## Panics
    ///
    /// If the size of the buffer is bigger than the shared buffer
    pub fn copy_from_shared_buffer(&self, buffer: &mut [u8]) {
        unsafe {
            self.copy_from_shared_buffer_raw(buffer.as_mut_ptr(), buffer.len());
        }
    }


    /// ## Safety
    ///
    /// Reads from the returned pointer must be volatile
    pub unsafe fn shared_buffer_raw_mut(&mut self) -> *mut [u8; GHCB_SHARED_BUF_SIZE] {
        ptr::from_mut(&mut self.shared_buffer)
    }

    pub fn set_shared_buffer(&mut self, value: [u8; GHCB_SHARED_BUF_SIZE]) {
        let ptr = ptr::from_mut(&mut self.shared_buffer);

        unsafe {
            ptr.write_volatile(value);
        }

        self.use_shared_buffer();
    }

    /// Copies the address of the shared buffer in the SW_SCRATCH field of the saved data and marks
    /// that field valid.
    pub fn use_shared_buffer(&mut self) {
        // let ptr: *const u8 = &self.shared_buffer[0];
        // self.save.sw_scratch = ptr as u64;
        self.save.set_field_valid(GhcbOtherField::ScratchAddress as usize);
    }


    pub fn set_field(&mut self, field: GhcbU64Field, value: u64) {
        unsafe {
            self.save.offset_mut_ptr::<u64>(field as usize).write_volatile(value);
        }
        self.save.set_field_valid(field as usize);
    }

    pub fn set_exit_code(&mut self, ghcb_exit_code: GhcbExitCode) {
        unsafe {
            self.save.offset_mut_ptr::<GhcbExitCode>(GhcbOtherField::ExitCode as usize).write_volatile(ghcb_exit_code);
        }
        self.save.set_field_valid(GhcbOtherField::ExitCode as usize);
    }

    pub fn get_field_if_valid(&self, field: GhcbU64Field) -> Option<u64> {
        if !self.save.is_field_valid(field as usize) {
            None
        } else {
            unsafe {
                Some(self.save.offset_ptr::<u64>(field as usize).read_volatile())
            }
        }
    }

    pub(super) fn set_phys_address(&mut self, frame: PhysFrame<Size4KiB>) {
        unsafe {
            // Setting the scratch address is "normal" but does not imply that it is valid, so we only set it
            ptr::from_mut(&mut self.save.sw_scratch).write_volatile(frame.start_address().add(GHCB_SCRATCH_OFFSET).as_u64());
        }
    }

    pub fn set_protocol_version(&mut self, protocol_version: u16) {
        unsafe {
            ptr::from_mut(&mut self.protocol_version).write_volatile(protocol_version);
        }
    }
}
