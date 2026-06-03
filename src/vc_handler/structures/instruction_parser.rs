use core::slice;
use core::mem::MaybeUninit;
use x86_64::VirtAddr;
use crate::vc_handler::structures::stack_frame::VCInterruptStackFrame;
use super::opcodes::instruction_prefix::RegisterExtensionPrefix;
use super::opcodes::{opcode, ExtendedRegister, Register};
use super::opcodes::{instruction_prefix};
use super::opcodes::opcode::KnownOpcode;

const MAX_INSTRUCTION_LENGTH: usize = 15;

/// The instruction that caused the exception to be raised
#[derive(Debug)]
pub struct InstructionData {
    base_ptr: *const u8,
    current_offset: usize,

    // Parsed data
    operand_size: OperandSize,
    address_size: OperandSize,
    repetition_mode: Option<InstructionRepetitionMode>,
    rex_prefix: Option<RegisterExtensionPrefix>,
    opcode: MaybeUninit<KnownOpcode>,

    modrm_read: bool,
    displacement_read: bool,
    immediate_read: bool
}


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InstructionRepetitionMode {
    /// Applied to a string operation: repeats the operation until the rCX register equals 0.
    ///
    /// Applied to a compare-string or scan-string operation: repeats the operation until the rCX
    /// register equals 0 or the ZF flag is cleared to 0
    RepZ,

    /// Applied to a compare-string or scan-string operation: repeats the operation until the rCX
    /// register equals 0 or the ZF flag is set to 1
    RepNZ
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OperandSize {
    Size16Bits, Size32Bits, Size64Bits
}

impl InstructionData {
    pub fn new(instr_ptr: *const ()) -> Self {
        let mut res = Self {
            base_ptr: instr_ptr as *const u8,
            current_offset: 0,

            repetition_mode: None,
            rex_prefix: None,
            opcode: MaybeUninit::uninit(),

            // Default values for long mode
            operand_size: OperandSize::Size32Bits,
            address_size: OperandSize::Size64Bits,

            // Safety trackers
            modrm_read: false,
            displacement_read: false,
            immediate_read: false,
        };

        res.parse_prefixes();
        res.parse_opcode();

        res
    }

    fn parse_prefixes(&mut self) {
        // Liberally inspired by EDK2: https://github.com/tianocore/edk2/blob/master/OvmfPkg/Library/CcExitLib/CcInstruction.c#L307
        while self.current_offset < MAX_INSTRUCTION_LENGTH {
            // Peek the next instruction byte. We don't consume it as it may be an instruction byte.
            let opcode = unsafe { self.peek_byte() };

            // Parse 64-bit register extension prefix
            if let Some(prefix) = RegisterExtensionPrefix::from_opcode(opcode) {
                if prefix.contains(RegisterExtensionPrefix::BitW) {
                    if self.operand_size == OperandSize::Size16Bits {
                        panic!("error while parsing instruction: cannot combine override operand size and REX prefixes for same instruction!")
                    }
                    self.operand_size = OperandSize::Size64Bits;
                }

                self.rex_prefix = Some(prefix);

                self.next();
                continue;
            }

            // Parse other opcodes
            match opcode {
                instruction_prefix::OVERRIDE_SEGMENT_CS | instruction_prefix::OVERRIDE_SEGMENT_DS | instruction_prefix::OVERRIDE_SEGMENT_ES | instruction_prefix::OVERRIDE_SEGMENT_FS | instruction_prefix::OVERRIDE_SEGMENT_GS | instruction_prefix::OVERRIDE_SEGMENT_SS => {
                    /* No OP: In 64-bit mode, we can ignore segment modifiers */
                }
                instruction_prefix::LOCK => { /* No OP */ }
                instruction_prefix::VEX_MAP_1 | instruction_prefix::VEX_MAP_2 | instruction_prefix::XOP => {
                    panic!("error while parsing instruction: unhandled long mode extended instructions!")
                }
                instruction_prefix::OVERRIDE_OPERAND_SIZE => {
                    // Always in 64bits mode
                    self.operand_size = OperandSize::Size16Bits;
                }
                instruction_prefix::OVERRIDE_ADDRESS_SIZE => {
                    // Always in 64bits mode
                    self.address_size = OperandSize::Size32Bits;
                }
                instruction_prefix::REP_NZ => {
                    self.repetition_mode = Some(InstructionRepetitionMode::RepNZ)
                }
                instruction_prefix::REP_Z => {
                    self.repetition_mode = Some(InstructionRepetitionMode::RepZ)
                }
                _ => {
                    // Not a prefix - it's an opcode
                    return;
                }
            }

            self.next();
        }

        panic!("error while parsing instruction: instruction too long");
    }

    /// Read the ModRM part of an instruction.
    ///
    /// Returns a tuple (reg, r/mem).
    ///
    /// The r/mem part must be a memory location, otherwise our handler should not have been called
    ///
    /// ## Safety
    ///
    /// You must make sure that the instruction you're reading has a ModRM component and that it
    /// has not been extracted yet.
    #[inline]
    pub unsafe fn parse_modrm_data(&mut self, frame: &VCInterruptStackFrame) -> (ExtendedRegister, VirtAddr) {
        assert_eq!(self.modrm_read, false);
        self.modrm_read = true;

        unsafe {
            let rm_info = ModRmInfo::read_from_instruction(self);
            rm_info.finalize_from_instruction(self, frame)
        }
    }

    fn parse_opcode(&mut self) {
        // By default, an OPCODE is 1 byte, except when extended
        let first_byte = unsafe { self.read_byte() };

        let opcode = if first_byte == opcode::TWO_BYTE_ESCAPE {
            let second_byte = unsafe { self.read_byte() };
            if second_byte == opcode::TWO_BYTE_ESCAPE {
                panic!("error while parsing instruction: unhandled 3DNow instruction!")
            }
            ((first_byte as u16) << 8) | (second_byte as u16)
        } else {
            first_byte as u16
        };

        let known_opcode = KnownOpcode::from_bits(opcode);
        if known_opcode == KnownOpcode::UnknownOpcode {
            panic!("#VC on unhandled opcode {opcode:x}, at eip={:p}!", self.base_ptr);
        }
        self.opcode = MaybeUninit::new(known_opcode);
    }


    #[inline(always)]
    fn next(&mut self) {
        self.advance(1)
    }

    #[inline(always)]
    unsafe fn read_byte(&mut self) -> u8 {
        let value = unsafe { self.peek_byte() };
        self.advance(1);
        value
    }

    #[inline(always)]
    fn read_bytes(&mut self, len: usize) -> &[u8] {
        assert!(len + self.current_offset <= MAX_INSTRUCTION_LENGTH);

        let value = unsafe { slice::from_raw_parts(self.current_ptr(), len) };
        self.advance(len);
        value
    }

    #[inline(always)]
    pub unsafe fn read_displacement(&mut self, len: usize) -> &[u8] {
        assert_eq!(self.displacement_read, false);
        assert!(len <= 8);
        self.displacement_read = true;

        if len == 8 {
            // If a displacement of size 8 is read, there cannot be an immediate
            self.immediate_read = true;
        }

        self.read_bytes(len)
    }

    #[inline(always)]
    pub unsafe fn read_immediate(&mut self, len: usize) -> &[u8] {
        assert_eq!(self.immediate_read, false);
        self.immediate_read = true;
        self.read_bytes(len)
    }

    pub fn operation(&mut self) -> KnownOpcode {
        unsafe {
            // SAFETY: we know this is safe as `parse_instruction` is called while initializing this struct
            self.opcode.assume_init()
        }
    }

    fn advance(&mut self, offset: usize) {
        assert!(self.current_offset + offset <= MAX_INSTRUCTION_LENGTH);
        self.current_offset += offset;
    }

    pub fn size(&self) -> usize {
        self.current_offset
    }

    pub unsafe fn current_ptr(&self) -> *const u8 {
        assert!(self.current_offset < MAX_INSTRUCTION_LENGTH);
        unsafe {
            self.base_ptr.add(self.current_offset)
        }
    }

    pub fn base_ptr(&self) -> *const u8 {
        self.base_ptr
    }

    pub unsafe fn peek_byte(&self) -> u8 {
        unsafe { *self.current_ptr() }
    }

    pub fn repetition_mode(&self) -> Option<& InstructionRepetitionMode> {
        self.repetition_mode.as_ref()
    }

    pub fn rex_prefix(&self) -> Option<& RegisterExtensionPrefix> {
        self.rex_prefix.as_ref()
    }

    pub fn operand_size(&self) -> OperandSize {
        self.operand_size
    }

    pub fn address_size(&self) -> OperandSize {
        self.address_size
    }
}


#[derive(Debug)]
struct ModRmInfo(InstructionModRMData, Option<InstructionSIBData>);

impl ModRmInfo {
    unsafe fn read_from_instruction(instruction_data: &mut InstructionData) -> ModRmInfo {
        let modrm = InstructionModRMData(unsafe { instruction_data.read_byte() });

        let sib = if modrm.register_or_memory() == RegisterOrMemory::SIB {
            // Parse SIB byte
            Some(InstructionSIBData(unsafe { instruction_data.read_byte() }))
        } else {
            None
        };

        ModRmInfo(modrm, sib)
    }

    fn get_displacement_bytes(&self) -> u8 {
        match self.0.modrm_mode() {
            ModRmMode::MemoryNoDisplacement
                if self.0.register_or_memory() == RegisterOrMemory::RelativeToInstruction ||
                 self.1.as_ref().is_some_and(|sib| sib.base_raw() == 0b101) => 4,
            ModRmMode::MemoryNoDisplacement => 0,
            ModRmMode::Memory8BitsDisplacement => 1,
            ModRmMode::Memory32BitsDisplacement => 4,
            ModRmMode::Register => 0,
        }
    }

    /// Return the register (reg) part of this ModRM byte, possibly extended by the REX byte in the
    /// instruction prefix
    pub fn extended_register(&self, instruction_data: &InstructionData) -> ExtendedRegister {
        Self::extend_reg(instruction_data.rex_prefix(), self.0.register())
    }

    /// Returns the RM part (register or memory) from this ModRM byte.
    ///
    /// In some cases, this function will read additional bytes from instruction data. For that
    /// reason, it takes ownership of this object and drops it, to avoid multiple reads.
    pub unsafe fn finalize_rm_from_instruction(self, instruction_data: &mut InstructionData, frame: &VCInterruptStackFrame) -> VirtAddr {
        let target = self.0.register_or_memory();

        if let RegisterOrMemory::Register(_) = target {
            unreachable!("unreachable: a page fault was triggered on an operation targetting a register");
            /* return ExtendedRMOperand::Register(
                Self::extend_reg(instruction_data.rex_prefix(), target),
            ) */
        };


        let displacement_bytes = self.get_displacement_bytes();
        let displacement = if self.get_displacement_bytes() > 0 {
            let bytes = unsafe { instruction_data.read_displacement(displacement_bytes as usize) };

            if displacement_bytes == 1 {
                bytes[0] as u64
            } else if displacement_bytes == 2 {
                (bytes[1] as u64) << 8 | (bytes[0] as u64)
            } else if displacement_bytes == 4 {
                (bytes[3] as u64) << 24 | (bytes[2] as u64) << 16 | (bytes[1] as u64) << 8 | (bytes[1] as u64)
            } else {
                panic!("invalid displacement bytes");
            }
        } else { 0 };

        let target_addr: u64 = if let Some(sib) = self.1 {
            let base = Self::extend_base_or_rm(instruction_data.rex_prefix(), sib.base());
            let base = if base.0 == Register::Rbp && !base.1 {
                if self.0.modrm_mode() == ModRmMode::MemoryNoDisplacement {
                    frame.exception.instruction_pointer.as_u64()
                } else {
                    frame.registers.rbp
                }
            } else {
                base.get_register(frame)
            };

            if sib.index_raw() == 0b100 {
                // AMD Programmer Manual, vol 3, page 20, table 1-12, footnote 1
                // "Register specification is null. The scale*index portion of the indexed register-indirect effec-
                // tive address is set to 0" [if SIB.index == 0b100]
                base + displacement
            } else {
                let index = Self::extend_sib_index(instruction_data.rex_prefix(), sib.index());
                let index = index.get_register(frame);

                let scale = sib.scale() as u64;

                (scale * index) + base + displacement
            }
        } else {
            // Simple mode
            displacement + match target {
                RegisterOrMemory::MemoryOffset(reg) => {
                    Self::extend_base_or_rm(instruction_data.rex_prefix(), reg).get_register(frame)
                }
                RegisterOrMemory::RelativeToInstruction => {
                    frame.exception.instruction_pointer.as_u64()
                }
                RegisterOrMemory::Register(_) => unreachable!(),
                RegisterOrMemory::SIB => unreachable!(),
            }
        };

        VirtAddr::new(target_addr)
    }

    pub unsafe fn finalize_from_instruction(self, instruction_data: &mut InstructionData, frame: &VCInterruptStackFrame) -> (ExtendedRegister, VirtAddr) {
        let register = Self::extend_reg(instruction_data.rex_prefix(), self.0.register());
        let register_or_memory = unsafe { self.finalize_rm_from_instruction(instruction_data, frame) };

        (register, register_or_memory)
    }

    /// Return the register given as parameter, extended if the ExtendModRmReg bit is set in REX
    fn extend_reg(rex_prefix: Option<&RegisterExtensionPrefix>, base: Register) -> ExtendedRegister {
        ExtendedRegister(base, rex_prefix.is_some_and(|rex| rex.contains(RegisterExtensionPrefix::ExtendModRmReg)))
    }

    /// Return the register given as parameter, extended if the ExtendSibIndex bit is set in REX
    fn extend_sib_index(rex_prefix: Option<&RegisterExtensionPrefix>, base: Register) -> ExtendedRegister {
        ExtendedRegister(base, rex_prefix.is_some_and(|rex| rex.contains(RegisterExtensionPrefix::ExtendSibIndex)))
    }

    /// Return the register given as parameter, extended if the B bit is set in REX (targets the base in an SIB case, or the R/M register in a non SIB case)
    fn extend_base_or_rm(rex_prefix: Option<&RegisterExtensionPrefix>, base: Register) -> ExtendedRegister {
        ExtendedRegister(base, rex_prefix.is_some_and(|rex| rex.contains(RegisterExtensionPrefix::BitB)))
    }
}


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum RegisterOrMemory {
    Register(Register),
    MemoryOffset(Register),
    RelativeToInstruction,
    SIB
}

const REGISTERS: [Register; 8] = [
    Register::Rax, Register::Rcx, Register::Rdx, Register::Rbx,
    Register::Rsp, Register::Rbp, Register::Rsi, Register::Rdi
];

impl InstructionModRMData {

    /// Returns the mode of this modifier
    pub fn modrm_mode(&self) -> ModRmMode {
        MODES[(self.0 >> 6 & 0x3) as usize]
    }

    /// Returns the register indicated by this modifier
    ///
    /// This may be unused for some instructions
    pub fn register(&self) -> Register  {
        REGISTERS[(self.0 >> 3 & 0x7) as usize]
    }

    /// Returns the 3 least significant bits of the ModRM operand modifier ("r/m")
    pub fn register_or_memory(&self) -> RegisterOrMemory {
        let mode = self.modrm_mode();
        let bits = self.0 & 0x7;
        let register = REGISTERS[bits as usize];

        if mode == ModRmMode::Register {
            RegisterOrMemory::Register(register)
        } else if mode == ModRmMode::MemoryNoDisplacement && bits == 0b101 {
            RegisterOrMemory::RelativeToInstruction
        } else if bits == 0b100 {
            RegisterOrMemory::SIB
        } else {
            RegisterOrMemory::MemoryOffset(register)
        }
    }
}


/// See AMD programmers manual volume 3, section 1.4
#[repr(transparent)]
#[derive(Debug)]
struct InstructionSIBData(pub u8);

impl InstructionSIBData {
    pub fn scale(&self) -> u8 {
        let bits = (self.0 >> 6) & 0b11;

        match bits {
            0b00 => 1,
            0b01 => 2,
            0b10 => 4,
            0b11 => 8,
            _ => unreachable!()
        }
    }
    pub fn index(&self) -> Register {
        REGISTERS[self.index_raw() as usize]
    }

    pub fn index_raw(&self) -> u8 {
        self.0 >> 3 & 0x7
    }

    pub fn base(&self) -> Register {
        REGISTERS[self.base_raw() as usize]
    }

    pub fn base_raw(&self) -> u8 {
        self.0 & 0x7
    }
}


/// See AMD programmers manual volume 3, section 1.4
#[repr(transparent)]
#[derive(Debug)]
struct InstructionModRMData(pub u8);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
enum ModRmMode {
    MemoryNoDisplacement,
    Memory8BitsDisplacement,
    Memory32BitsDisplacement,
    Register
}

const MODES: [ModRmMode; 4] = [
    ModRmMode::MemoryNoDisplacement, // 00
    ModRmMode::Memory8BitsDisplacement, // 01
    ModRmMode::Memory32BitsDisplacement, // 10
    ModRmMode::Register, // 11
];
