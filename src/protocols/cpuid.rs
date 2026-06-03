use core::arch::x86_64::CpuidResult;
use x86_64::registers::control::{Cr4, Cr4Flags};
use x86_64::registers::xcontrol::XCr0;
use crate::protocols::GhcbProtocolRequest;
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::exit_codes::GhcbExitCode;
use crate::structures::ghcb_page::GhcbU64Field;

pub struct CpuIdRequest {
    leaf: u32,
    subleaf: u32,
    xcr0: u64
}

fn read_xcr0() -> u64 {
    let cr4_flags = Cr4::read();

    if cr4_flags.contains(Cr4Flags::OSXSAVE) {
        XCr0::read_raw()
    } else {
        0
    }
}

impl CpuIdRequest {
    #[inline(always)]
    pub fn for_leaf(leaf: u32) -> CpuIdRequest {
        // This frame regulates the AMD SEV features - we don't want to get it in an insecure way
        assert_ne!(
            leaf, 0x8000_001f,
            "Encrypted memory capabilities CPUID function must be secured by hypervisor in CPUID page!"
        );

        CpuIdRequest { leaf, subleaf: 0, xcr0: 0 }
    }

    #[inline(always)]
    pub fn with_subleaf(self, subleaf: u32) -> CpuIdRequest {
        CpuIdRequest { subleaf, ..self }
    }

    #[inline(always)]
    pub fn with_xcr0(self, xcr0: u64) -> CpuIdRequest {
        CpuIdRequest { xcr0, ..self }
    }
}

impl GhcbProtocolRequest for CpuIdRequest {
    type Response = CpuidResult;

    fn execute_request(self, ghcb: &mut GhcbRequestExecutor) -> Self::Response {
        ghcb.raw().clear();
        ghcb.raw().set_field(GhcbU64Field::Rax, self.leaf as u64);
        ghcb.raw().set_field(GhcbU64Field::Rcx, self.subleaf as u64);

        if self.leaf == 0x0000_000d {
            ghcb.raw().set_field(GhcbU64Field::XCr0, self.xcr0);
        }

        ghcb.checked_vmgexit(GhcbExitCode::CPUID, 0, 0);

        CpuidResult {
            eax: ghcb.raw().get_field_if_valid(GhcbU64Field::Rax).expect("cpuid: missing EAX field") as u32,
            ebx: ghcb.raw().get_field_if_valid(GhcbU64Field::Rbx).expect("cpuid: missing EAX field") as u32,
            ecx: ghcb.raw().get_field_if_valid(GhcbU64Field::Rcx).expect("cpuid: missing EAX field") as u32,
            edx: ghcb.raw().get_field_if_valid(GhcbU64Field::Rdx).expect("cpuid: missing EAX field") as u32,
        }
    }
}