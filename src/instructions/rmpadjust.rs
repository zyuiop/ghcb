use bitfield_struct::bitfield;
use core::arch::asm;
use x86_64::structures::paging::{Page, PageSize};

#[bitfield(u64)]
pub struct RmpAdjustment {
    #[bits(8)]
    pub target_vmpl: u8,

    #[bits(8)]
    pub target_permissions_mask: u8,

    #[bits(1)]
    pub vmsa: bool,

    #[bits(47)]
    _reserved: u64,
}

impl RmpAdjustment {
    /// Returns an RMP Adjustment that marks the page as a VMSA, with given VM Privilege Level
    pub fn new_vmsa(target_vmpl: u8) -> Self {
        Self::new().with_vmsa(true).with_target_vmpl(target_vmpl)
    }
}

/// Issue an RMP adjust instruction for the given page
pub unsafe fn rmpadjust<S: PageSize>(page: Page<S>, adjustment: RmpAdjustment) {
    unsafe {
        let rmpadjust = adjustment.0;
        let guest_addr_ret = page.start_address().as_u64();
        asm!("rmpadjust",
            in("rax") guest_addr_ret,
            in("rcx") S::SIZE,
            in("rdx") rmpadjust,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::RmpAdjustment;

    #[test]
    fn check_bitfield_is_correct() {
        let expected = (1 << 16) | 1u64;
        let check = RmpAdjustment::new().with_vmsa(true).with_target_vmpl(1);

        assert_eq!(check.0, expected);
    }
}
