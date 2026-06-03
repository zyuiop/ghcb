use crate::vc_handler::structures::stack_frame::VCInterruptStackFrame;

pub mod opcode {
    use bitfield_struct::bitenum;

    const fn long_instr(byte: u8) -> u16 {
        (TWO_BYTE_ESCAPE as u16) << 8 | byte as u16
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(u16)]
    #[bitenum]
    pub enum KnownOpcode {
        /// Read a byte in the AL register from the port specified by an 8-bit immediate value
        IoInByteImm = 0xE4,

        /// Read a (double) word in the (E)AX register from the port specified by an 8-bit immediate value
        IoInByteDx = 0xEC,


        /// Read a byte in the AL register from the port specified in the DX register
        IoInWordsImm = 0xE5,


        /// Read a (double) word in the (E)AX register from the port specified in the DX register
        IoInWordsDx = 0xED,

        IoInsByte = 0x6C,
        IoInsWords = 0x6D,

        /// Writes the byte in the AL register to the port specified by an 8-bit immediate value
        IoOutByteImm = 0xE6,

        /// Writes the (double) word in the AL register to the port specified by an 8-bit immediate value
        IoOutWordsImm = 0xE7,

        /// Writes the byte in the AL register to the port specified in the DX register
        IoOutByteDx = 0xEE,

        /// Writes the (double) word in the AL register to the port specified in the DX register
        IoOutWordsDx = 0xEF,


        IoOutsByte = 0x6E,
        IoOutsWords = 0x6F,

        CPUID = long_instr(0xa2),

        WRMSR = long_instr(0x30),
        RDMSR = long_instr(0x32),

        /// MOV r/m8, r8
        ///
        /// Moves the content of ModRM.reg to ModRM.rm (MMIO write), byte sized
        MovRmRegByte = 0x88,
        /// MOV r/m16, r16
        ///
        /// Moves the content of ModRM.reg to ModRM.rm (MMIO write)
        MovRmReg = 0x89,

        /// MOV r8, r/m8
        ///
        /// Moves the content of ModRM.rm to ModRM.reg (MMIO read), byte sized
        MovRegRmByte = 0x8A,
        /// MOV r16, r/m16
        ///
        /// Moves the content of ModRM.rm to ModRM.reg (MMIO read)
        MovRegRm = 0x8B,

        /// MOVZ r8, r/m8
        ///
        /// Moves the content of ModRM.rm to ModRM.reg (MMIO read), byte sized, with zero extension
        MovzRegRmByte = long_instr(0xB6),
        /// MOVZ r16, r/m16
        ///
        /// Moves the content of ModRM.rm to ModRM.reg (MMIO read), with zero extension
        MovzRegRm = long_instr(0xB7),

        /// MOV r/m8, imm8
        ///
        /// Moves the immediate value to ModRM.rm, byte sized
        MovRmImmByte = 0xC6,
        /// MOV r/m16, imm16
        ///
        /// Moves the immediate value to ModRM.rm
        MovRmImm = 0xC7,

        /// OR r/m8, imm8
        ///
        /// Stores the bitwise OR with the immediate value in ModRM.rm, byte sized
        OrRmImmByte = 0x80,
        /// OR r/m16, imm16
        ///
        /// Stores the bitwise OR with the immediate value in ModRM.rm
        OrRmImm = 0x81,

        /// TEST r/m8, imm8
        ///
        /// Tests the immediate value against ModRM.rm, byte sized.
        ///
        /// See [https://www.felixcloutier.com/x86/test]
        TestRmByte = 0xF6,
        /// TEST r/m16, imm16
        ///
        /// Tests the immediate value against ModRM.rm.
        ///
        /// See [https://www.felixcloutier.com/x86/test]
        TestRm = 0xF7,

        /// CMP r/m16, imm8
        ///
        /// Compares an immediate byte with a value
        CmpImm = 0x83,

        #[fallback]
        UnknownOpcode = 0xFF,
    }

    /// Implies an extended opcode (2-3 bytes)
    pub const TWO_BYTE_ESCAPE: u8 = 0x0F;

}


pub mod instruction_prefix {
    pub const OVERRIDE_SEGMENT_CS: u8 = 0x2E;
    pub const OVERRIDE_SEGMENT_DS: u8 = 0x3E;
    pub const OVERRIDE_SEGMENT_ES: u8 = 0x26;
    pub const OVERRIDE_SEGMENT_SS: u8 = 0x36;
    pub const OVERRIDE_SEGMENT_FS: u8 = 0x64;
    pub const OVERRIDE_SEGMENT_GS: u8 = 0x65;

    /// In 64-bit mode, override operand size from the default (32bits) to 16 bits
    pub const OVERRIDE_OPERAND_SIZE: u8 = 0x66;

    /// In 64-bit mode, override address size from the default (64bits) to 32 bits
    pub const OVERRIDE_ADDRESS_SIZE: u8 = 0x67;

    pub const REP_NZ: u8 = 0xF2;
    pub const REP_Z: u8 = 0xF3;

    pub const REX_PREFIX_START: u8 = 0x40;
    pub const REX_PREFIX_END: u8 = 0x4F;

    /// Extended Instruction
    pub const VEX_MAP_1: u8 = 0xC5;


    /// Extended Instruction
    pub const VEX_MAP_2: u8 = 0xC4;

    /// Extended Instruction
    pub const XOP: u8 = 0x8F;

    pub const LOCK: u8 = 0xF0;


    bitflags! {
        #[derive(Clone, Copy, Debug)]
        pub struct RegisterExtensionPrefix: u8 {
            const BitB = 1 << 0;
            const ExtendSibIndex = 1 << 1;
            const ExtendModRmReg = 1 << 2;

            /// If enabled, operand size is 64-niz
            const BitW = 1 << 3;
        }
    }

    impl RegisterExtensionPrefix {
        pub fn from_opcode(opcode: u8) -> Option<RegisterExtensionPrefix> {
            if opcode >= REX_PREFIX_START && opcode <= REX_PREFIX_END {
                Some(RegisterExtensionPrefix::from_bits_truncate(opcode))
            } else {
                None
            }
        }
    }
}


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Register {
    Rax, Rcx, Rdx, Rbx,
    Rsp, Rbp, Rsi, Rdi
}

impl Register {
    fn get_register(&self, frame: &VCInterruptStackFrame, x64_extended: bool) -> u64 {
        if x64_extended {
            match self {
                Register::Rax => frame.registers.r8,
                Register::Rcx => frame.registers.r9,
                Register::Rdx => frame.registers.r10,
                Register::Rbx => frame.registers.r11,
                Register::Rsp => frame.registers.r12,
                Register::Rbp => frame.registers.r13,
                Register::Rsi => frame.registers.r14,
                Register::Rdi => frame.registers.r15
            }
        } else {
            match self {
                Register::Rax => frame.registers.rax,
                Register::Rcx => frame.registers.rcx,
                Register::Rdx => frame.registers.rdx,
                Register::Rbx => frame.registers.rbx,
                Register::Rsp => frame.exception.stack_pointer.as_u64(),
                Register::Rbp => frame.registers.rbp,
                Register::Rsi => frame.registers.rsi,
                Register::Rdi => frame.registers.rdi
            }
        }
    }

    fn as_ptr(&self, frame: &VCInterruptStackFrame, x64_extended: bool) -> *const u8 {
        if x64_extended {
            match self {
                Register::Rax => &frame.registers.r8 as *const _ as *const u8,
                Register::Rcx => &frame.registers.r9 as *const _ as *const u8,
                Register::Rdx => &frame.registers.r10 as *const _ as *const u8,
                Register::Rbx => &frame.registers.r11 as *const _ as *const u8,
                Register::Rsp => &frame.registers.r12 as *const _ as *const u8,
                Register::Rbp => &frame.registers.r13 as *const _ as *const u8,
                Register::Rsi => &frame.registers.r14 as *const _ as *const u8,
                Register::Rdi => &frame.registers.r15 as *const _ as *const u8
            }
        } else {
            match self {
                Register::Rax => &frame.registers.rax as *const _ as *const u8,
                Register::Rcx => &frame.registers.rcx as *const _ as *const u8,
                Register::Rdx => &frame.registers.rdx as *const _ as *const u8,
                Register::Rbx => &frame.registers.rbx as *const _ as *const u8,
                Register::Rsp => &frame.exception.stack_pointer as *const _ as *const u8,
                Register::Rbp => &frame.registers.rbp as *const _ as *const u8,
                Register::Rsi => &frame.registers.rsi as *const _ as *const u8,
                Register::Rdi => &frame.registers.rdi as *const _ as *const u8
            }
        }
    }

    fn as_mut_ptr(&self, frame: &mut VCInterruptStackFrame, x64_extended: bool) -> *mut u8 {
        let reference = self.get_register_mut(frame, x64_extended);
        reference as *mut _ as *mut u8
    }

    fn get_register_mut<'a>(&self, frame: &'a mut VCInterruptStackFrame, x64_extended: bool) -> &'a mut u64 {
        if x64_extended {
            match self {
                Register::Rax => &mut frame.registers.r8,
                Register::Rcx => &mut frame.registers.r9,
                Register::Rdx => &mut frame.registers.r10,
                Register::Rbx => &mut frame.registers.r11,
                Register::Rsp => &mut frame.registers.r12,
                Register::Rbp => &mut frame.registers.r13,
                Register::Rsi => &mut frame.registers.r14,
                Register::Rdi => &mut frame.registers.r15
            }
        } else {
            match self {
                Register::Rax => &mut frame.registers.rax,
                Register::Rcx => &mut frame.registers.rcx,
                Register::Rdx => &mut frame.registers.rdx,
                Register::Rbx => &mut frame.registers.rbx,
                Register::Rsp => unsafe { ((&mut frame.exception.stack_pointer) as *mut _ as *mut u64).as_mut().unwrap() },
                Register::Rbp => &mut frame.registers.rbp,
                Register::Rsi => &mut frame.registers.rsi,
                Register::Rdi => &mut frame.registers.rdi
            }
        }
    }

}


/// Represents a register which may have been "increased" via a x64 register extension command
pub struct ExtendedRegister(pub Register, pub bool);

impl ExtendedRegister {
    pub fn get_register(&self, frame: &VCInterruptStackFrame) -> u64 {
        // TODO: implement extended mode
        self.0.get_register(frame, self.1)
    }

    /// Selects this register as a read-only byte slice, of requested size
    pub fn as_slice<'a>(&self, frame: &'a VCInterruptStackFrame, size: usize) -> &'a [u8] {
        assert!(size < size_of::<u64>());

        unsafe {
            core::slice::from_raw_parts(self.0.as_ptr(frame, self.1), size)
        }
    }

    pub fn as_mut_slice<'a>(&self, frame: &'a mut VCInterruptStackFrame, size: usize) -> &'a mut [u8] {
        assert!(size < size_of::<u64>());

        unsafe {
            core::slice::from_raw_parts_mut(self.0.as_mut_ptr(frame, self.1), size_of::<u64>())
        }
    }

    pub fn get_register_mut<'a>(&self, frame: &'a mut VCInterruptStackFrame) -> &'a mut u64 {
        // TODO: implement extended mode
        self.0.get_register_mut(frame, self.1)
    }
}

pub struct DisplacedMemoryLocation {
    pub displacement_bytes: u8,
    pub base_register: Option<ExtendedRegister>
}

pub enum BaseMemoryLocation {
    RelativeToInstruction,
    Register(ExtendedRegister),
    SIB {
        /// If absent, relative to instruction
        base_register: Option<ExtendedRegister>,
        /// If absent, use 0 as value
        index: Option<ExtendedRegister>,
    }
}

/*
pub enum ExtendedRMOperand {
    Register(ExtendedRegister),
    Memory(VirtAddr)
}

impl ExtendedRMOperand {
    /// Returns a read-only slice over this memory or register
    ///
    /// ## Safety
    ///
    /// Caller must ensure that size fits inside the target location (register or memory)
    pub unsafe fn as_slice<'a>(&self, frame: &'a VCInterruptStackFrame, size: usize) -> &'a [u8] {
        match self {
            ExtendedRMOperand::Register(r) => r.as_slice(frame, size),
            ExtendedRMOperand::Memory(addr) => unsafe {
                core::slice::from_raw_parts(addr.as_ptr::<u8>(), size)
            }
        }
    }

    /// Returns a read/write slice over this memory or register
    ///
    /// ## Safety
    ///
    /// Caller must ensure that size fits inside the target location (register or memory)
    pub unsafe fn as_mut_slice<'a>(&self, frame: &'a mut VCInterruptStackFrame, size: usize) -> &'a mut [u8] {
        match self {
            ExtendedRMOperand::Register(r) => r.as_mut_slice(frame, size),
            ExtendedRMOperand::Memory(addr) => unsafe {
                core::slice::from_raw_parts_mut(addr.as_mut_ptr::<u8>(), size)
            }
        }
    }
}
*/