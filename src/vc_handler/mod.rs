use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::ChannelManager;
use crate::vc_handler::structures::instruction_parser::InstructionData;
use crate::vc_handler::structures::stack_frame::VCInterruptStackFrame;

pub mod structures;
pub mod builder;
pub mod exits;

/// Represents a handler for a specific #VC exception.
pub trait VcHandler {
    fn handle(&self,
              frame: &mut VCInterruptStackFrame,
              instruction_data: &mut InstructionData);
}

/// Represents a handler for a specific #VC exception.
pub trait GhcbVcHandler {
    type ChannelManager: ChannelManager;

    fn handle_with_ghcb(&self, frame: &mut VCInterruptStackFrame, instruction_data: &mut InstructionData, ghcb: &mut GhcbRequestExecutor);
}

impl<T: GhcbVcHandler> VcHandler for T {
    fn handle(&self, frame: &mut VCInterruptStackFrame, instruction_data: &mut InstructionData) {
        T::ChannelManager::get_channel().with_ghcb(|mut ghcb| {
            self.handle_with_ghcb(frame, instruction_data, &mut ghcb)
        })
    }
}
