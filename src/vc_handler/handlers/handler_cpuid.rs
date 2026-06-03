use crate::protocols::GhcbProtocolRequest;
use crate::protocols::cpuid::CpuIdRequest;
use crate::structures::ChannelManager;
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::snp_cpuid_page::CPUIDPage;
use crate::vc_handler::GhcbVcHandler;
use crate::vc_handler::structures::instruction_parser::InstructionData;
use crate::vc_handler::structures::opcodes::opcode::KnownOpcode;
use crate::vc_handler::structures::stack_frame::VCInterruptStackFrame;
use core::marker::PhantomData;
use x86_64::registers::control::{Cr4, Cr4Flags};
use x86_64::registers::xcontrol::XCr0;

pub trait CpuIdPageAccessor {
    fn with_cpuid_page<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&CPUIDPage) -> R;
}

#[derive(Debug)]
pub struct CpuIdHandler<'a, C: ChannelManager, S: CpuIdPageAccessor> {
    cpuid: &'a S,
    _phantom: PhantomData<C>,
}

impl<'a, C: ChannelManager, S: CpuIdPageAccessor> CpuIdHandler<'a, C, S> {
    #[inline(always)]
    pub const fn new(cpuid_accessor: &'a S) -> Self {
        Self {
            cpuid: cpuid_accessor,
            _phantom: PhantomData,
        }
    }
}
// Restricted CPUID calls

impl<'a, C: ChannelManager, S: CpuIdPageAccessor> GhcbVcHandler for CpuIdHandler<'a, C, S> {
    type ChannelManager = C;

    fn handle_with_ghcb(
        &self,
        frame: &mut VCInterruptStackFrame,
        idata: &mut InstructionData,
        ghcb: &mut GhcbRequestExecutor,
    ) {
        assert_eq!(idata.operation(), KnownOpcode::CPUID);

        // XCr0 is not always accessible: it must be enabled by a special Cr4 flag
        // Check the flag (and the need to use Xcr0) before accessing it
        let xcr0 = if frame.registers.rax == 0x0000_000d {
            let cr4_flags = Cr4::read();

            if cr4_flags.contains(Cr4Flags::OSXSAVE) {
                XCr0::read_raw()
            } else {
                0
            }
        } else {
            0
        };

        let found = self.cpuid.with_cpuid_page(|cpuid| {
            match cpuid.get_cpuid(
                (frame.registers.rax & 0xffff_ffff) as u32,
                (frame.registers.rcx & 0xffff_ffff) as u32,
                xcr0,
            ) {
                None => false,
                Some(existing) => {
                    frame.registers.rax = existing.eax as u64;
                    frame.registers.rbx = existing.ebx as u64;
                    frame.registers.rcx = existing.ecx as u64;
                    frame.registers.rdx = existing.edx as u64;

                    true
                }
            }
        });

        if found {
            return;
        }

        if frame.registers.rax == 0x8000_001f {
            // This frame regulates the AMD SEV features - we don't want to get it in an insecure way
            panic!(
                "Encrypted memory capabilities CPUID function must be secured by hypervisor in CPUID page!"
            );
        }

        let mut request = CpuIdRequest::for_leaf(frame.registers.rax as u32)
            .with_subleaf(frame.registers.rcx as u32);

        // Need XCR0 if CPUID 0000_000d
        if frame.registers.rax == 0x0000_000d {
            request = request.with_xcr0(xcr0);
        };

        let result = request.execute_request(ghcb);
        frame.registers.rax = result.eax as u64;
        frame.registers.rbx = result.ebx as u64;
        frame.registers.rcx = result.ecx as u64;
        frame.registers.rdx = result.edx as u64;
    }
}
