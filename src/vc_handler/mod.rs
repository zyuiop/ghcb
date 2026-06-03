use crate::structures::channel::GhcbRequestExecutor;
use crate::vc_handler::structures::instruction_parser::InstructionData;
use crate::vc_handler::structures::stack_frame::VCInterruptStackFrame;

pub mod structures;

pub mod internals;

/// Represents a handler for a specific #VC exception.
pub trait VcHandler {
    fn handle(&self,
              frame: &mut VCInterruptStackFrame,
              ghcb: &mut GhcbRequestExecutor,
              instruction_data: &mut InstructionData);
}