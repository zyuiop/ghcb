use crate::protocols::GhcbProtocolRequest;
use crate::protocols::ioio::{IoIoOperation, IoIoRequest};
use crate::structures::ChannelManager;
use crate::structures::channel::GhcbRequestExecutor;
use crate::vc_handler::GhcbVcHandler;
use crate::vc_handler::structures::instruction_parser::{InstructionData, OperandSize};
use crate::vc_handler::structures::opcodes::opcode::KnownOpcode;
use crate::vc_handler::structures::stack_frame::{SavedRegisters, VCInterruptStackFrame};
use core::marker::PhantomData;

#[derive(Debug)]
pub struct IoIoHandler<T: ChannelManager>(PhantomData<T>);

#[inline(always)]
unsafe fn cast_ref<T>(source_ref: &mut u64) -> &mut T {
    unsafe {
        let ptr = source_ref as *mut u64;
        let ptr = ptr.cast::<T>();
        ptr.as_mut().unwrap()
    }
}

impl<T: ChannelManager> IoIoHandler<T> {
    #[inline(always)]
    pub const fn new() -> Self {
        IoIoHandler(PhantomData)
    }

    fn call_from_instruction<'a>(
        instruction_data: &mut InstructionData,
        registers: &'a mut SavedRegisters,
    ) -> IoIoRequest<'a> {
        let operand_16bits = instruction_data.operand_size() == OperandSize::Size16Bits;

        let out = match instruction_data.operation() {
            KnownOpcode::IoInsByte | KnownOpcode::IoInsWords => IoIoRequest::new(
                (registers.rdx & 0xffff) as u16,
                unimplemented!("STRING operations unhandled!"),
            ),
            KnownOpcode::IoOutsByte | KnownOpcode::IoOutsWords => IoIoRequest::new(
                (registers.rdx & 0xffff) as u16,
                unimplemented!("STRING operations unhandled!"),
            )
            .with_segment(0x3),
            op @ (KnownOpcode::IoInByteImm
            | KnownOpcode::IoInWordsImm
            | KnownOpcode::IoOutByteImm
            | KnownOpcode::IoOutWordsImm) => {
                let port = unsafe { instruction_data.read_immediate(1)[0] as u16 };
                let op = match op {
                    KnownOpcode::IoInByteImm => {
                        IoIoOperation::ByteIn(unsafe { cast_ref(&mut registers.rax) })
                    }
                    KnownOpcode::IoInWordsImm if operand_16bits => {
                        IoIoOperation::WordIn(unsafe { cast_ref(&mut registers.rax) })
                    }
                    KnownOpcode::IoInWordsImm => {
                        IoIoOperation::DblWordIn(unsafe { cast_ref(&mut registers.rax) })
                    }
                    KnownOpcode::IoOutByteImm => {
                        IoIoOperation::ByteOut((registers.rax & 0xff) as u8)
                    }
                    KnownOpcode::IoOutWordsImm if operand_16bits => {
                        IoIoOperation::WordOut((registers.rax & 0xffff) as u16)
                    }
                    KnownOpcode::IoOutWordsImm => {
                        IoIoOperation::DblWordOut((registers.rax & 0xffff_fffff) as u32)
                    }
                    _ => unreachable!(),
                };

                IoIoRequest::new(port, op)
            }
            op @ (KnownOpcode::IoInByteDx
            | KnownOpcode::IoInWordsDx
            | KnownOpcode::IoOutByteDx
            | KnownOpcode::IoOutWordsDx) => {
                let port = (registers.rdx & 0xffff) as u16;
                let op = match op {
                    KnownOpcode::IoInByteDx => {
                        IoIoOperation::ByteIn(unsafe { cast_ref(&mut registers.rax) })
                    }
                    KnownOpcode::IoInWordsDx if operand_16bits => {
                        IoIoOperation::WordIn(unsafe { cast_ref(&mut registers.rax) })
                    }
                    KnownOpcode::IoInWordsDx => {
                        IoIoOperation::DblWordIn(unsafe { cast_ref(&mut registers.rax) })
                    }
                    KnownOpcode::IoOutByteDx => {
                        IoIoOperation::ByteOut((registers.rax & 0xff) as u8)
                    }
                    KnownOpcode::IoOutWordsDx if operand_16bits => {
                        IoIoOperation::WordOut((registers.rax & 0xffff) as u16)
                    }
                    KnownOpcode::IoOutWordsDx => {
                        IoIoOperation::DblWordOut((registers.rax & 0xffff_fffff) as u32)
                    }
                    _ => unreachable!(),
                };

                IoIoRequest::new(port, op)
            }
            other => {
                panic!("invalid ioio opcode {other:?}");
            }
        };

        // Determine address size
        /* out.flags.insert(match instruction_data.address_size() {
            Size::Size16Bits => IoIoExitFlags::ADDR_16B,
            Size::Size32Bits => IoIoExitFlags::ADDR_32B,
            Size::Size64Bits => IoIoExitFlags::ADDR_64B,
        }); */

        if instruction_data.repetition_mode().is_some() {
            panic!("invalid ioio repetition mode");
        }

        out
    }
}

impl<T: ChannelManager> GhcbVcHandler for IoIoHandler<T> {
    type ChannelManager = T;

    fn handle_with_ghcb(
        &self,
        frame: &mut VCInterruptStackFrame,
        instruction_data: &mut InstructionData,
        ghcb: &mut GhcbRequestExecutor,
    ) {
        let info = Self::call_from_instruction(instruction_data, &mut frame.registers);
        info.execute_request(ghcb);
    }
}
