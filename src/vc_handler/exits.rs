//! See table/appendix C-1 in AMD Programme Manual Vol. 2 (Doc ID 24593)

use bitfield_struct::bitenum;

#[bitenum]
#[repr(i64)]
#[derive(Debug, Copy, Clone)]
pub enum SvmInterceptCode {
    /// Read of CR0
    Cr0Read = 0x0,
    /// Read of CR1
    Cr1Read = 0x1,
    /// Read of CR2
    Cr2Read = 0x2,
    /// Read of CR3
    Cr3Read = 0x3,
    /// Read of CR4
    Cr4Read = 0x4,
    /// Read of CR5
    Cr5Read = 0x5,
    /// Read of CR6
    Cr6Read = 0x6,
    /// Read of CR7
    Cr7Read = 0x7,
    /// Read of CR8
    Cr8Read = 0x8,
    /// Read of CR9
    Cr9Read = 0x9,
    /// Read of CR10
    Cr10Read = 0xa,
    /// Read of CR11
    Cr11Read = 0xb,
    /// Read of CR12
    Cr12Read = 0xc,
    /// Read of CR13
    Cr13Read = 0xd,
    /// Read of CR14
    Cr14Read = 0xe,
    /// Read of CR15
    Cr15Read = 0xf,
    /// Write of CR0
    Cr0Write = 0x10,
    /// Write of CR1
    Cr1Write = 0x11,
    /// Write of CR2
    Cr2Write = 0x12,
    /// Write of CR3
    Cr3Write = 0x13,
    /// Write of CR4
    Cr4Write = 0x14,
    /// Write of CR5
    Cr5Write = 0x15,
    /// Write of CR6
    Cr6Write = 0x16,
    /// Write of CR7
    Cr7Write = 0x17,
    /// Write of CR8
    Cr8Write = 0x18,
    /// Write of CR9
    Cr9Write = 0x19,
    /// Write of CR10
    Cr10Write = 0x1a,
    /// Write of CR11
    Cr11Write = 0x1b,
    /// Write of CR12
    Cr12Write = 0x1c,
    /// Write of CR13
    Cr13Write = 0x1d,
    /// Write of CR14
    Cr14Write = 0x1e,
    /// Write of CR15
    Cr15Write = 0x1f,
    /// Read of DCR0
    Dr0Read = 0x20,
    /// Read of DCR1
    Dr1Read = 0x21,
    /// Read of DCR2
    Dr2Read = 0x22,
    /// Read of DCR3
    Dr3Read = 0x23,
    /// Read of DCR4
    Dr4Read = 0x24,
    /// Read of DCR5
    Dr5Read = 0x25,
    /// Read of DCR6
    Dr6Read = 0x26,
    /// Read of DCR7
    Dr7Read = 0x27,
    /// Read of DCR8
    Dr8Read = 0x28,
    /// Read of DCR9
    Dr9Read = 0x29,
    /// Read of DR10
    Dr10Read = 0x2a,
    /// Read of DR11
    Dr11Read = 0x2b,
    /// Read of DR12
    Dr12Read = 0x2c,
    /// Read of DR13
    Dr13Read = 0x2d,
    /// Read of DR14
    Dr14Read = 0x2e,
    /// Read of DR15
    Dr15Read = 0x2f,
    /// Write of DCR0
    Dr0Write = 0x30,
    /// Write of DCR1
    Dr1Write = 0x31,
    /// Write of DCR2
    Dr2Write = 0x32,
    /// Write of DCR3
    Dr3Write = 0x33,
    /// Write of DCR4
    Dr4Write = 0x34,
    /// Write of DCR5
    Dr5Write = 0x35,
    /// Write of DCR6
    Dr6Write = 0x36,
    /// Write of DCR7
    Dr7Write = 0x37,
    /// Write of DCR8
    Dr8Write = 0x38,
    /// Write of DCR9
    Dr9Write = 0x39,
    /// Write of DR10
    Dr10Write = 0x3a,
    /// Write of DR11
    Dr11Write = 0x3b,
    /// Write of DR12
    Dr12Write = 0x3c,
    /// Write of DR13
    Dr13Write = 0x3d,
    /// Write of DR14
    Dr14Write = 0x3e,
    /// Write of DR15
    Dr15Write = 0x3f,
    /// Exception vector 0
    EXCP0 = 0x40,
    /// Exception vector 1
    EXCP1 = 0x41,
    /// Exception vector 2
    EXCP2 = 0x42,
    /// Exception vector 3
    EXCP3 = 0x43,
    /// Exception vector 4
    EXCP4 = 0x44,
    /// Exception vector 5
    EXCP5 = 0x45,
    /// Exception vector 6
    EXCP6 = 0x46,
    /// Exception vector 7
    EXCP7 = 0x47,
    /// Exception vector 8
    EXCP8 = 0x48,
    /// Exception vector 9
    EXCP9 = 0x49,
    /// Exception vector 10
    EXCP10 = 0x4a,
    /// Exception vector 11
    EXCP11 = 0x4b,
    /// Exception vector 12
    EXCP12 = 0x4c,
    /// Exception vector 13
    EXCP13 = 0x4d,
    /// Exception vector 14
    EXCP14 = 0x4e,
    /// Exception vector 15
    EXCP15 = 0x4f,
    /// Exception vector 16
    EXCP16 = 0x50,
    /// Exception vector 17
    EXCP17 = 0x51,
    /// Exception vector 18
    EXCP18 = 0x52,
    /// Exception vector 19
    EXCP19 = 0x53,
    /// Exception vector 20
    EXCP20 = 0x54,
    /// Exception vector 21
    EXCP21 = 0x55,
    /// Exception vector 22
    EXCP22 = 0x56,
    /// Exception vector 23
    EXCP23 = 0x57,
    /// Exception vector 24
    EXCP24 = 0x58,
    /// Exception vector 25
    EXCP25 = 0x59,
    /// Exception vector 26
    EXCP26 = 0x5a,
    /// Exception vector 27
    EXCP27 = 0x5b,
    /// Exception vector 28
    EXCP28 = 0x5c,
    /// Exception vector 29
    EXCP29 = 0x5d,
    /// Exception vector 30
    EXCP30 = 0x5e,
    /// Exception vector 31
    EXCP31 = 0x5f,
    /// Physical INTR (maskable interrupt).
    INTR = 0x60,
    /// Physical NMI.
    NMI = 0x61,
    /// Physical SMI (the EXITINFO1 field provides more information).
    SMI = 0x62,
    /// Physical INIT.
    INIT = 0x63,
    /// Virtual INTR.
    VINTR = 0x64,
    /// Write of CR0 changed bits other than CR0.TS or CR0.MP.
    Cr0SelWrite = 0x65,
    /// Read of IDTR.
    IdtrRead = 0x66,
    /// Read of GDTR.
    GdtrRead = 0x67,
    /// Read of LDTR.
    LdtrRead = 0x68,
    /// Read of TR.
    TrRead = 0x69,
    /// Write of IDTR.
    IdtrWrite = 0x6A,
    /// Write of GDTR.
    GdtrWrite = 0x6B,
    /// Write of LDTR.
    LdtrWrite = 0x6C,
    /// Write of TR.
    TrWrite = 0x6D,
    /// RDTSC instruction.
    RDTSC = 0x6E,
    /// RDPMC instruction.
    RDPMC = 0x6F,
    /// PUSHF instruction.
    PUSHF = 0x70,
    /// POPF instruction.
    POPF = 0x71,
    /// CPUID instruction.
    CPUID = 0x72,
    /// RSM instruction.
    RSM = 0x73,
    /// IRET instruction.
    IRET = 0x74,
    /// Software interrupt (INTn instructions).
    SWINT = 0x75,
    /// INVD instruction.
    INVD = 0x76,
    /// PAUSE instruction.
    PAUSE = 0x77,
    /// HLT instruction
    HLT = 0x78,
    /// INVLPG instructions.
    INVLPG = 0x79,
    /// INVLPGA instruction.
    INVLPGA = 0x7A,
    /// IN or OUT accessing protected port (the EXITINFO1 field provides more information).
    IOIO = 0x7B,
    /// RDMSR or WRMSR access to protected MSR.
    MSR = 0x7C,
    /// Task switch.
    TaskSwitch = 0x7D,
    /// FP legacy handling enabled, and processor is frozen in an x87/mmx instruction waiting for an interrupt.
    FerrFreeze = 0x7E,
    /// Shutdown
    SHUTDOWN = 0x7F,
    /// VMRUN instruction.
    VMRUN = 0x80,
    /// VMMCALL instruction.
    VMMCALL = 0x81,
    /// VMLOAD instruction.
    VMLOAD = 0x82,
    /// VMSAVE instruction.
    VMSAVE = 0x83,
    /// STGI instruction.
    STGI = 0x84,
    /// CLGI instruction.
    CLGI = 0x85,
    /// SKINIT instruction.
    SKINIT = 0x86,
    /// RDTSCP instruction.
    RDTSCP = 0x87,
    /// ICEBP instruction.
    ICEBP = 0x88,
    /// WBINVD or WBNOINVD instruction.
    WBINVD = 0x89,
    /// MONITOR or MONITORX instruction.
    MONITOR = 0x8A,
    /// MWAIT or MWAITX instruction.
    Mwait = 0x8B,
    /// MWAIT or MWAITX instruction, if monitor hardware is armed.
    MwaitConditional = 0x8C,
    /// RDPRU instruction.
    RDPRU = 0x8E,
    /// XSETBV instruction.
    XSETBV = 0x8D,
    /// Write of EFER MSR (occurs after guest instruction finishes).
    EferWriteTrap = 0x8F,
    /// Write of CR0 (occurs after guest instruction finishes).
    Cr0WriteTrap = 0x90,
    /// Write of CR1 (occurs after guest instruction finishes).
    Cr1WriteTrap = 0x91,
    /// Write of CR2 (occurs after guest instruction finishes).
    Cr2WriteTrap = 0x92,
    /// Write of CR3 (occurs after guest instruction finishes).
    Cr3WriteTrap = 0x93,
    /// Write of CR4 (occurs after guest instruction finishes).
    Cr4WriteTrap = 0x94,
    /// Write of CR5 (occurs after guest instruction finishes).
    Cr5WriteTrap = 0x95,
    /// Write of CR6 (occurs after guest instruction finishes).
    Cr6WriteTrap = 0x96,
    /// Write of CR7 (occurs after guest instruction finishes).
    Cr7WriteTrap = 0x97,
    /// Write of CR8 (occurs after guest instruction finishes).
    Cr8WriteTrap = 0x98,
    /// Write of CR9 (occurs after guest instruction finishes).
    Cr9WriteTrap = 0x99,
    /// Write of CR10 (occurs after guest instruction finishes).
    Cr10WriteTrap = 0x9a,
    /// Write of CR11 (occurs after guest instruction finishes).
    Cr11WriteTrap = 0x9b,
    /// Write of CR12 (occurs after guest instruction finishes).
    Cr12WriteTrap = 0x9c,
    /// Write of CR13 (occurs after guest instruction finishes).
    Cr13WriteTrap = 0x9d,
    /// Write of CR14 (occurs after guest instruction finishes).
    Cr14WriteTrap = 0x9e,
    /// Write of CR15 (occurs after guest instruction finishes).
    Cr15WriteTrap = 0x9f,
    /// INVLPGB instruction.
    INVLPGB = 0xA0,
    /// Illegal INVLPGB instruction.
    InvlpgbIllegal = 0xA1,
    /// INVPCID instruction.
    INVPCID = 0xA2,
    /// MCOMMIT instruction.
    MCOMMIT = 0xA3,
    /// TLBSYNC instruction.
    TLBSYNC = 0xA4,
    /// Bus lock while Bus Lock Threshold Counter value is 0.
    BUSLOCK = 0xA5,
    /// HLT instruction if a virtual interrupt is not pending
    IdleHlt = 0xA6,
    /// Nested paging: host-level page fault occurred (EXITINFO1 contains fault error code; EXITINFO2 contains the guest physical address causing the fault).
    NPF = 0x400,
    /// AVIC—Virtual IPI delivery not completed. See "AVIC IPI Delivery Not Completed" on page 580 for EXITINFO1–2 definitions.
    AvicIncompleteIpi = 0x401,
    /// AVIC—Attempted access by guest to vAPIC register not handled by AVIC hardware. See "AVIC Access to Un- accelerated vAPIC register" on page 581 for EXITINFO1–2 definitions.
    AvicNoaccel = 0x402,
    /// VMGEXIT instruction.
    VMGEXIT = 0x403,

    /// See AMD Programmer's Manual vol. 2 (doc id 24593), section §15.36.10
    /// > A failure of the page validation check results in a #VC with error code PAGE_NOT_VALIDATED
    PageNotValidated = 0x404,

    Unused = 0xF000_0000,

    #[fallback]
    /// Invalid guest state in VMCB
    Invalid = -1,
    /// BUSY bit was set in the VMSA (see "Interrupt Injection Restrictions" on page 614)
    Busy = -2,
    /// The sibling thread is not in an idle state (see "Side-Channel Protection" on page 615).
    IdleRequired = -3,
    /// Invalid PMC state (see “Performance Monitoring Counter Virtualization” on page N).
    InvalidPMC = -4,
}
