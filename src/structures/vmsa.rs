use bitflags::Flags;
use core::mem::{offset_of, MaybeUninit};
use core::ops::BitAnd;
use core::ptr;
use core::ptr::NonNull;
use static_assertions::const_assert_eq;
use x86_64::registers::control::{Cr0Flags, Cr3Flags, Cr4, Cr4Flags, EferFlags};
use x86_64::registers::debug::{Dr6Flags, Dr7Flags};
use x86_64::registers::rflags::RFlags;
use x86_64::registers::xcontrol::XCr0Flags;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{Page, Size4KiB};
use crate::instructions::rmpadjust::{rmpadjust, RmpAdjustment};
use crate::sev_status::{SevStatusFlags, SevStatusMsr};
use crate::util::{OwnedPtr, OwnedPtrWithPhysAddr};

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct SegmentRegister {
    selector: u16,
    attribute: u16,
    limit: u32,
    _reserved: u32,
    base: u32,
}

const CS_ATTR_PRESENT: u16 = 1 << 7;

impl Default for SegmentRegister {
    fn default() -> Self {
        Self {
            // Default values from the AMD Programmers' Manual
            selector: 0,
            base: 0,
            _reserved: 0,
            limit: 0xffff,
            attribute: CS_ATTR_PRESENT | 0b10010,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct PerfCtl {
    perf_ctl: u64,
    perf_ctr: u64,
}

bitflags! {
    #[repr(transparent)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct SnpFeatures: u64 {
        const SNP_ACTIVE = 1 << 0;
        const V_TOM = 1 << 1;
        const REFLECT_VC = 1 << 2;
        const RESTRICTED_INJECTION = 1 << 3;
        const ALTERNATE_INJECTION = 1 << 4;
        const DEBUG_VIRTUALIZATION = 1 << 5;
        const PREVENT_HOST_IBS = 1 << 6;
        const BTB_ISOLATION = 1 << 7;
        const VMP_ISSS = 1 << 8;
        const SEURE_TSC = 1 << 9;
        const VMGEXIT_PARAMETER = 1 << 10;
        const PMC_VIRTUALIZATION = 1 << 11;
        const IBS_VIRTUALIZATION = 1 << 12;
        const VMSA_REGISTER_PROTECTION = 1 << 14;
        const SMT_PROTECTION = 1 << 15;
        const SECURE_AVIC = 1 << 16;
        const IBPB_ON_ENTRY = 1 << 21;
    }
}

impl From<SevStatusFlags> for SnpFeatures {
    fn from(status: SevStatusFlags) -> SnpFeatures {
        SnpFeatures::from_bits_truncate(status.bits() >> 2)
    }
}

macro_rules! init_sr {
    ($sr: expr) => {
        $sr.limit = 0xffff;
        $sr.attribute = CS_ATTR_PRESENT | 0b10010;
    };
}

impl VMSaveArea {
    /// Recommended way to declare a VMSaveArea. Prepare a pointer, then simply call init on it.
    pub fn init(value: &mut MaybeUninit<Self>) -> &mut Self {
        unsafe {
            // Zero memory before doing anything
            value.as_mut_ptr().write_bytes(0, 1);
        }

        let mut value = unsafe {
            value.assume_init_mut()
        };

        init_sr!(value.es);
        init_sr!(value.cs);
        init_sr!(value.ss);
        init_sr!(value.ds);
        init_sr!(value.fs);
        init_sr!(value.gs);

        value.gdtr.limit = 0xffff;
        value.idtr.limit = 0xffff;
        value.ldtr.limit = 0xffff;
        value.ldtr.attribute = CS_ATTR_PRESENT | 0b0010;
        value.tr.limit = 0xffff;
        value.tr.attribute = CS_ATTR_PRESENT | 0b0011;

        value.gdtr.limit = 0xffff;
        value.gdtr.attribute = CS_ATTR_PRESENT | 0b10010;

        value.cr3 = Cr3Flags::empty();
        value.cr0 = Cr0Flags::from_bits_retain(0x6000_0010);
        value.dr7 = Dr7Flags::from_bits_retain(0x400);
        value.dr6 = Dr6Flags::from_bits_retain(0xffff_0ff0);
        value.rflags = RFlags::from_bits_retain(0x2);
        value.xcr0 = XCr0Flags::from_bits_retain(1);

        // CR4: only forward MACHINE_CHECK_EXCEPTION
        value.cr4 = Cr4::read().bitand(Cr4Flags::MACHINE_CHECK_EXCEPTION);
        value.efer = EferFlags::SECURE_VIRTUAL_MACHINE_ENABLE;

        value.x87_fcw = 0x0040;
        value.x87_ftw = 0x5555;
        value.mx_csr = 0x1f80;
        value.snp_features = SnpFeatures::from(SevStatusMsr::read());

        value
    }
}


impl Default for VMSaveArea {
    /// WARNING: This method will absolutely destroy your stack
    fn default() -> Self {
        // AMD Programmers Manual, vol 2, 14.1.3, Processor Initialization State
        // See linux kernel: coco/sev/core.c, wakeup_cpu_via_vmgexit (https://github.com/torvalds/linux/blob/master/arch/x86/coco/sev/core.c#L1180)

        Self {
            cr3: Cr3Flags::empty(),
            cr0: Cr0Flags::from_bits_retain(0x6000_0010),
            dr7: Dr7Flags::from_bits_retain(0x400),
            dr6: Dr6Flags::from_bits_retain(0xffff_0ff0),
            rflags: RFlags::from_bits_retain(0x2),
            xcr0: XCr0Flags::from_bits_retain(1),
            // CR4: only forward MACHINE_CHECK_EXCEPTION
            cr4: Cr4::read().bitand(Cr4Flags::MACHINE_CHECK_EXCEPTION),
            efer: EferFlags::SECURE_VIRTUAL_MACHINE_ENABLE,

            gdtr: SegmentRegister {
                attribute: 0,
                ..Default::default()
            },
            idtr: SegmentRegister {
                attribute: 0,
                ..Default::default()
            },
            ldtr: SegmentRegister {
                attribute: CS_ATTR_PRESENT | 0b0010,
                ..Default::default()
            },
            tr: SegmentRegister {
                attribute: CS_ATTR_PRESENT | 0b0011,
                ..Default::default()
            },

            // x87 FP state
            x87_fcw: 0x0040, // control word
            x87_ftw: 0x5555, // tag word
            mx_csr: 0x1f80,

            snp_features: SnpFeatures::from(SevStatusMsr::read()),

            ..Default::default()
        }
    }
}

impl VMSaveArea {
    /// Set-up the VMSA to boot a new core.
    /// Provided address is the initial jump address for the core.
    pub fn set_start_instr_ptr(&mut self, init_address: VirtAddr) {
        // See linux kernel: coco/sev/core.c, wakeup_cpu_via_vmgexit (https://github.com/torvalds/linux/blob/master/arch/x86/coco/sev/core.c#L1180)

        let ip = init_address.as_u64();

        // Set code segment register
        let sipi_vector = ip >> 16;
        self.cs.base = (sipi_vector << 16) as u32;
        self.cs.selector = 8u16;
        self.cs.limit = 0xffff;
        self.cs.attribute = CS_ATTR_PRESENT | 0b11010; // SVM_S, CODE, READ

        // Set RIP
        self.rip = ip & 0xffff; // 16 last bits of selected segment
    }

    pub fn snp_features(&self) -> SnpFeatures {
        self.snp_features
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct VMSaveArea {
    es: SegmentRegister,
    cs: SegmentRegister,
    ss: SegmentRegister,
    ds: SegmentRegister,
    fs: SegmentRegister,
    gs: SegmentRegister,
    gdtr: SegmentRegister,
    ldtr: SegmentRegister,
    idtr: SegmentRegister,
    tr: SegmentRegister,

    pl_ssp: [u64; 4],
    u_cet: u64,
    _reserved_0: u16, // Documentation is wrong: this is a word and not a dword!

    vmpl: u8,
    cpl: u8,
    _reserved_1: u32,

    efer: EferFlags,
    _reserved_2: u64,

    perf_ctls: [PerfCtl; 6],
    xss: u64,
    cr4: Cr4Flags,
    cr3: Cr3Flags,
    cr0: Cr0Flags,
    dr7: Dr7Flags,
    dr6: Dr6Flags,
    rflags: RFlags,

    rip: u64,
    /// DR0 to DR3
    dr: [u64; 4],
    /// DR0 to DR3
    dr_addr_mask: [u64; 4],

    instr_retired_ctr: u64,
    perf_ctr_global_stats: u64,
    perf_ctr_global_ctl: u32,
    _reserved_3: u32,

    rsp: u64,
    s_cet: u64,
    ssp: u64,
    isst_addr: u64,
    rax: u64,
    star: u64,
    lstar: u64,
    cstar: u64,
    sfmask: u64,
    kernel_gs_base: u64,

    sysenter_cs: u64,
    sysenter_esp: u64,
    sysenter_eip: u64,

    cr2: u64,

    _reserved_4: [u64; 4], // 32 bytes

    g_pat: u64,
    dbgctl: u64,
    br_from: u64,
    br_to: u64,

    last_except_from: u64,
    last_except_to: u64,
    dbg_extn_cfg: u64,

    // 64 bytes - not 72, the documentation is once again wrong
    _reserved_5: [u64; 8],

    spec_ctrl: u64,
    pkru: u32,
    tsc_aux: u32,
    guest_tsc_scale: u64,
    guest_tsc_offset: u64,
    reg_prot_nonce: u64,

    rcx: u64,
    rdx: u64,
    rbx: u64,

    secure_avic_ctl: u64,

    rbp: u64,
    rsi: u64,
    rdi: u64,
    /// registers r8 to r15
    x64_registers: [u64; 8],
    _reserved_6: u128,

    guest_exitinfo1: u64,
    guest_exitinfo2: u64,
    guest_exitintinfo: u64,
    guest_nrip: u64,

    snp_features: SnpFeatures,
    vintr_ctrl: u64,
    guest_exit_code: u64,
    virtual_tom: u64,
    tlb_id: u64,
    pcpu_id: u64,
    event_inj: u64,
    xcr0: XCr0Flags,
    _reserved_7: u128,
    x87_dp: u64,
    mx_csr: u32,
    x87_ftw: u16,
    x87_fsw: u16,
    x87_fcw: u16,
    x87_fop: u16,
    x87_ds: u16,
    x87_cs: u16,
    x87_rip: u64,

    fpreg_x87: [u64; 10],
    fpreg_xmm: [u64; 32],
    fpreg_ymm: [u64; 32],
    lbr_stack: [u64; 32],
    lbr_select: u64,
    ibs_fetch_ctl: u64,
    ibs_fetch_linaddr: u64,
    ibs_op_ctl: u64,
    ibs_op_rip: u64,
    ibs_op_data: [u64; 3],
    ibs_dc_linaddr: u64,
    bp_ibstgt_rip: u64,
    ic_ibs_extd_ctl: u64,
}

// Assertions to ensure VMSA offsets match the specifications
const_assert_eq!(offset_of!(VMSaveArea, cs), 0x10);
const_assert_eq!(offset_of!(VMSaveArea, efer), 0xD0);
const_assert_eq!(offset_of!(VMSaveArea, pl_ssp), 0xA0);
const_assert_eq!(offset_of!(VMSaveArea, u_cet), 0xC0);
const_assert_eq!(offset_of!(VMSaveArea, vmpl), 0xCA);
const_assert_eq!(offset_of!(VMSaveArea, cpl), 0xCB);
const_assert_eq!(offset_of!(VMSaveArea, efer), 0xD0);
const_assert_eq!(offset_of!(VMSaveArea, xss), 0x140);
const_assert_eq!(offset_of!(VMSaveArea, rip), 0x178);
const_assert_eq!(offset_of!(VMSaveArea, rsp), 0x1D8);
const_assert_eq!(offset_of!(VMSaveArea, cr2), 0x240);
const_assert_eq!(offset_of!(VMSaveArea, g_pat), 0x268);
const_assert_eq!(offset_of!(VMSaveArea, br_to), 0x280);
const_assert_eq!(offset_of!(VMSaveArea, dbg_extn_cfg), 0x298);
const_assert_eq!(offset_of!(VMSaveArea, spec_ctrl), 0x2E0);
const_assert_eq!(offset_of!(VMSaveArea, rbp), 0x328);
const_assert_eq!(offset_of!(VMSaveArea, guest_exitinfo1), 0x390);

pub type AllocatedVMSaveArea = OwnedPtrWithPhysAddr<VMSaveArea>;

impl AllocatedVMSaveArea {
    /// Initializes given pointer as a VMSaveArea page and registers it (RMPAdjust)
    ///
    /// ## Safety
    ///
    /// The pointer must be valid and point to memory big enough to accomodate a VMSaveArea.
    /// The pointer must be unique.
    pub unsafe fn from_uninit(
        allocated_ptr: *mut MaybeUninit<VMSaveArea>,
        phys_addr: PhysAddr
    ) -> Self {
        let ptr = VMSaveArea::init(allocated_ptr.as_mut().unwrap());
        let ptr = unsafe {
            Self::new(NonNull::from_mut(ptr), phys_addr)
        };

        // Register/RMPAdjust
        unsafe {
            rmpadjust::<Size4KiB>(
                Page::from_start_address(ptr.virt_addr()).unwrap(),
                RmpAdjustment::new_vmsa(1)
            )
        }

        ptr
    }
}
