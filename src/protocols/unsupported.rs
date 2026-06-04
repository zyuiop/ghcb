use crate::protocols::GhcbProtocolRequest;
use crate::structures::ChannelManager;
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::exit_codes::GhcbExitCode;
use core::arch::asm;

/// An exit event that causes a VM crash, but with more details than the MSR equivalent
#[repr(transparent)]
pub struct UnsupportedExit(pub u64);

impl GhcbProtocolRequest for UnsupportedExit {
    type Response = ();

    fn execute_request(self, ghcb: &mut GhcbRequestExecutor) -> Self::Response {
        ghcb.checked_vmgexit(GhcbExitCode::UnsupportedEvent, self.0, 0)
    }
}

/// Terminates the VM with a custom exit code
pub fn exit_error<T: ChannelManager>(code: u64) {
    unsafe { T::get_channel().with_ghcb_force(|mut ghcb| ghcb.terminate(code)) }
}

impl GhcbRequestExecutor<'_> {
    pub fn terminate(&mut self, code: u64) -> ! {
        UnsupportedExit(code).execute_request(self);

        // In case the termination returns, we enter an infinite halt loop
        loop {
            unsafe { asm!("hlt") }
        }
    }
}
