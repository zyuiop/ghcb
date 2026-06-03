use core::arch::asm;
use core::fmt::Debug;
use core::marker::PhantomData;
use x86_64::addr::{PhysAddr, VirtAddr};
use crate::protocols::mmio::{MmioRead, MmioWrite};
use crate::protocols::GhcbProtocolRequest;
use crate::structures::channel::GhcbRequestExecutor;
use x86_64::registers::rflags::RFlags;
use x86_64::structures::paging::mapper::TranslateResult;
use x86_64::structures::paging::Translate;
use crate::structures::ChannelManager;
use crate::vc_handler::GhcbVcHandler;
use crate::vc_handler::structures::instruction_parser::{InstructionData, OperandSize};
use crate::vc_handler::structures::opcodes::opcode::KnownOpcode;
use crate::vc_handler::structures::stack_frame::VCInterruptStackFrame;

#[derive(Debug)]
pub struct MmioHandler<'a, C: ChannelManager, T: Translate> {
	translate: &'a T,
	_phantom: PhantomData<C>,
}

impl<'a, C: ChannelManager, T: Translate> MmioHandler<'a, C, T> {
    #[inline(always)]
    pub const fn new(translate: &'a T) -> Self {
        Self {
            translate,
            _phantom: PhantomData,
        }
    }
}

macro_rules! util_asm_test {
    ($typ:ty, $rc:tt, $imm:expr, $rv:expr, $out:expr) => {{
        let sz = size_of::<$typ>();
        let temp = <$typ>::from_le_bytes(($rv[0..sz]).try_into().unwrap());
        let imm = <$typ>::from_le_bytes(($imm[0..sz]).try_into().unwrap());
        unsafe { asm!(
            "pushfq",
            "test {}, {}",
            "pushfq",
            "pop {}",
            "popfq",
            in($rc) temp,
            in($rc) imm,
            out(reg) $out,
            options(preserves_flags)
        ) };
    }};
}

macro_rules! util_asm_cmp_imm8 {
    ($typ:ty, $imm8:expr, $rv:expr, $out:expr) => {{
        let sz = size_of::<$typ>();
        let rv = <$typ>::from_le_bytes(($rv[0..sz]).try_into().unwrap());
        let imm = $imm8 as $typ; // Sign extend immediate value
        unsafe { asm!(
            "pushfq",
            "cmp {}, {}",
            "pushfq",
            "pop {}",
            "popfq",
            in(reg) rv,
            in(reg) imm,
            out(reg) $out,
            options(preserves_flags)
        ) };
    }};
}

impl<'a, C: ChannelManager, T: Translate> MmioHandler<'a, C, T> {
	#[inline(always)]
	fn map_address(&self, addr: VirtAddr) -> PhysAddr {
		let translate_result = self.translate.translate(addr);

		match translate_result {
			TranslateResult::NotMapped | TranslateResult::InvalidFrameAddress(_) => {
				panic!("Unable to determine the physical address of 0x{addr:X}");
			}
			TranslateResult::Mapped { frame, offset, .. } => {
				PhysAddr::new((frame.start_address() + offset).as_u64())
			}
		}
	}

}

impl<'a, C: ChannelManager, T: Translate> GhcbVcHandler for MmioHandler<'a, C, T> {
	type ChannelManager = C;

	fn handle_with_ghcb(
		&self,
		frame: &mut VCInterruptStackFrame,
		instruction_data: &mut InstructionData,
		ghcb: &mut GhcbRequestExecutor,
	) {

		// Read opcode, ignoring first byte (0f) if present
		let opcode = instruction_data.operation();

		// Reference in Edk2: https://github.com/tianocore/edk2/blob/master/OvmfPkg/Library/CcExitLib/CcExitVcHandler.c#L163
		let mut size = match instruction_data.operand_size() {
			OperandSize::Size16Bits => 2,
			OperandSize::Size32Bits => 4,
			OperandSize::Size64Bits => 8,
		};

		// Check for 8bit opcodes
		if (opcode as u8) & 1 == 0 {
			// 8 bit opcodes are always even, non-8bit opcodes are never even (see below)
			size = 1;
		}

		match opcode {
			KnownOpcode::MovRmRegByte | KnownOpcode::MovRmReg => {
				// MOV reg/mem to reg (write)
				let (register, address) = unsafe { instruction_data.parse_modrm_data(frame) };

				unsafe {
					MmioWrite::new(
						register.as_slice(frame, size),
						self.map_address(address),
					).execute_request(ghcb)
				}
			}
			KnownOpcode::MovRegRmByte | KnownOpcode::MovRegRm => {
				// MOV reg to reg/mem (read)
				let (register, address) = unsafe { instruction_data.parse_modrm_data(frame) };

				unsafe {
					MmioRead::new(
						self.map_address(address),
						register.as_mut_slice(frame, size),
					).execute_request(ghcb)
				}
			}
			KnownOpcode::MovzRegRmByte | KnownOpcode::MovzRegRm => {
				// MOVZX regx, reg/memX
				// Read with zero extension
				let (register, address) = unsafe { instruction_data.parse_modrm_data(frame) };
				let size = if opcode == KnownOpcode::MovzRegRmByte {
					1
				} else {
					2
				};

				unsafe {
					MmioRead::new(
						self.map_address(address),
						register.as_mut_slice(frame, size),
					).execute_request(ghcb);
				}

				if size == 1 {
					*register.get_register_mut(frame) &= 0xff;
				} else {
					*register.get_register_mut(frame) &= 0xffff;
				}
			}
			KnownOpcode::MovRmImmByte | KnownOpcode::MovRmImm => {
				// MOV imm to reg/mem (write)
				let (_, address) = unsafe { instruction_data.parse_modrm_data(frame) };
				let immediate = unsafe { instruction_data.read_immediate(size) };

				unsafe {
					MmioWrite::new(immediate, self.map_address(address)).execute_request(ghcb);
				}
			}
			KnownOpcode::OrRmImmByte | KnownOpcode::OrRmImm => {
				// OR operation: we will need both a read and a write... this looks a bit inefficient :(
				let (_, address) = unsafe { instruction_data.parse_modrm_data(frame) };
				let address = self.map_address(address);

				let immediate = unsafe { instruction_data.read_immediate(size) };
				let mut temp = [0u8; 8];

				unsafe {
					MmioRead::new(address, &mut temp[0..size]).execute_request(ghcb);
				}
				for offset in 0..size {
					temp[offset] |= immediate[offset]
				}

				// Update flags manually
				frame.exception.cpu_flags.set(RFlags::OVERFLOW_FLAG, false);
				frame.exception.cpu_flags.set(RFlags::CARRY_FLAG, false);
				frame
					.exception
					.cpu_flags
					.set(RFlags::ZERO_FLAG, u64::from_le_bytes(temp) == 0);
				frame
					.exception
					.cpu_flags
					.set(RFlags::PARITY_FLAG, temp[0].count_ones() & 0x1 == 0); // parity: least significant byte has even number of ones
				frame
					.exception
					.cpu_flags
					.set(RFlags::SIGN_FLAG, temp[size - 1] >> 7 == 1); // sign: most significant bit is 1

				unsafe {
					MmioWrite::new(&temp[0..size], address).execute_request(ghcb);
				}
			}
			KnownOpcode::TestRmByte | KnownOpcode::TestRm => {
				// TEST operation: we read the value and compte the results of the TEST instruction
				let (_, address) = unsafe { instruction_data.parse_modrm_data(frame) };
				let immediate = unsafe { instruction_data.read_immediate(size) };

				let mut temp = [0u8; 8];
				unsafe {
					MmioRead::new(self.map_address(address), &mut temp[0..size]).execute_request(ghcb);
				}

				let mut flags = frame.exception.cpu_flags.bits();
				// Delegate the computation to assembly to make sure we do it correctly
				match size {
					1 => util_asm_test!(i8, reg_byte, &immediate, &temp, flags),
					2 => util_asm_test!(i16, reg, &immediate, &temp, flags),
					4 => util_asm_test!(i32, reg, &immediate, &temp, flags),
					8 => util_asm_test!(i64, reg, &immediate, &temp, flags),
					_ => unreachable!(),
				}
				frame.exception.cpu_flags = RFlags::from_bits_retain(flags);
			}
			KnownOpcode::CmpImm => {
				// TEST operation: we read the value and compte the results of the TEST instruction
				let (_, address) = unsafe { instruction_data.parse_modrm_data(frame) };
				let immediate = unsafe { instruction_data.read_immediate(1)[0].cast_signed() };

				let mut temp = [0u8; 8];
				unsafe {
					MmioRead::new(self.map_address(address), &mut temp[0..size]).execute_request(ghcb);
				}

				let mut flags = frame.exception.cpu_flags.bits();
				// Delegate the computation to assembly to make sure we do it correctly
				match size {
					2 => util_asm_cmp_imm8!(i16, immediate, &temp, flags),
					4 => util_asm_cmp_imm8!(i32, immediate, &temp, flags),
					8 => util_asm_cmp_imm8!(i64, immediate, &temp, flags),
					_ => unreachable!(),
				}
				frame.exception.cpu_flags = RFlags::from_bits_retain(flags);
			}
			other => {
				panic!("unhandled mmio opcode {other:?}")
			}
		}
	}
}
