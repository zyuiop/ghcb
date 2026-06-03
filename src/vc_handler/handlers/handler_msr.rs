use core::marker::PhantomData;
use crate::protocols::GhcbProtocolRequest;
use crate::protocols::msr::{ReadMsrRequest, WriteMsrRequest};
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::ChannelManager;
use crate::vc_handler::GhcbVcHandler;
use crate::vc_handler::structures::instruction_parser::InstructionData;
use crate::vc_handler::structures::opcodes::opcode::KnownOpcode;
use crate::vc_handler::structures::stack_frame::VCInterruptStackFrame;

#[derive(Debug)]
pub struct MsrHandler<T: ChannelManager>(PhantomData<T>);

impl<T: ChannelManager> MsrHandler<T> {
	#[inline(always)]
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

impl<T: ChannelManager> GhcbVcHandler for MsrHandler<T> {
	type ChannelManager = T;

	fn handle_with_ghcb(
		&self,
		frame: &mut VCInterruptStackFrame,
		instruction_data: &mut InstructionData,
		ghcb: &mut GhcbRequestExecutor,
	) {
		let opcode = instruction_data.operation();
		if opcode == KnownOpcode::WRMSR {
			WriteMsrRequest::new(
				frame.registers.rcx as u32,
				frame.registers.rdx as u32,
				frame.registers.rax as u32,
			)
			.execute_request(ghcb);
		} else if opcode == KnownOpcode::RDMSR {
			let (high, low) = ReadMsrRequest::new(frame.registers.rcx as u32)
				.execute_request(ghcb);

			frame.registers.rax = low as u64;
			frame.registers.rdx = high as u64;
		} else {
			panic!("invalid MSR opcode {opcode:?}")
		}
	}
}
