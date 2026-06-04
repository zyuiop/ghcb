use crate::protocols::GhcbProtocolRequest;
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::exit_codes::GhcbExitCode;
use crate::structures::ghcb_page::GhcbU64Field;
use crate::structures::vmsa::AllocatedVMSaveArea;
use bitfield_struct::{bitenum, bitfield};
use x86_64::VirtAddr;

#[bitenum]
#[repr(u16)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ApOperation {
    /// Initialize the CPU on the next INIT-SIPI
    /// Invalid when Restricted Injection is enabled
    CreateAddWait = 0,

    #[fallback]
    /// Initialize the CPU immediately
    CreateAddImmediate = 1,

    /// Remove the VMSA for the CPU, preventing it from running
    DestroyRemove = 2,
}

#[bitfield(u64)]
struct ApRequest {
    #[bits(16)]
    operation: ApOperation,

    #[bits(4)]
    vmpl: u8,

    #[bits(12)]
    _reserved: u16,

    apic_id: u32,
}

pub struct SnpApCreate {
    vmsa: AllocatedVMSaveArea,
    request: ApRequest,
    start_jump_addr: VirtAddr,
}

impl SnpApCreate {
    /// Request to initialize a new processor. The processor will wake up and jump to [start_jump_addr] when
    /// this request is executed.
    ///
    /// - `vmsa` is a previously allocated and initialized VMSA. Use [AllocatedVMSaveArea::from_uninit] after
    /// allocating memory to initialize it.
    /// - `apic_id` is the APIC ID of the CPU to initialize
    /// - `start_jump_addr` is the address at which the CPU will jump (will be written in the provided VMSA)
    pub fn new(vmsa: AllocatedVMSaveArea, apic_id: u32, start_jump_addr: VirtAddr) -> Self {
        Self {
            vmsa,
            request: ApRequest::new()
                .with_apic_id(apic_id)
                .with_operation(ApOperation::CreateAddImmediate),
            start_jump_addr,
        }
    }

    pub fn with_operation(self, operation: ApOperation) -> Self {
        Self {
            request: self.request.with_operation(operation),
            ..self
        }
    }

    pub fn with_vmpl(self, vmpl: u8) -> Self {
        Self {
            request: self.request.with_vmpl(vmpl),
            ..self
        }
    }
}

impl GhcbProtocolRequest for SnpApCreate {
    type Response = ();

    fn execute_request(mut self, ghcb: &mut GhcbRequestExecutor) -> Self::Response {
        self.vmsa.set_start_instr_ptr(self.start_jump_addr);

        ghcb.raw().clear();
        ghcb.raw()
            .set_field(GhcbU64Field::Rax, self.vmsa.snp_features().bits());
        ghcb.checked_vmgexit(
            GhcbExitCode::SnpApCreation,
            self.request.into_bits(),
            self.vmsa.phys_addr().as_u64(),
        );
    }
}
