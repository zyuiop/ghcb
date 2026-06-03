use crate::protocols::change_page_state::PageStateChangePageSize;
use core::arch::asm;
use x86_64::VirtAddr;
use x86_64::structures::paging::{PageSize, Size2MiB, Size4KiB};

pub fn pvalidate(page_size: PageStateChangePageSize, validate: bool, address: VirtAddr) {
    let result = do_pvalidate(page_size, validate, address);

    if result == Err(PvalidateError::EntrySizeMismatch)
        && page_size == PageStateChangePageSize::PageSize2MB
    {
        // Page is mapped as small pages
        let num_pages = Size2MiB::SIZE / Size4KiB::SIZE;
        let mut va = address;
        for _ in 0..num_pages {
            do_pvalidate(PageStateChangePageSize::PageSize4KB, validate, va).unwrap();
            va += Size4KiB::SIZE;
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum PvalidateError {
    InvalidInput,
    EntrySizeMismatch,
}

fn do_pvalidate(
    page_size: PageStateChangePageSize,
    validate: bool,
    address: VirtAddr,
) -> Result<(), PvalidateError> {
    let valid: u32 = if validate { 1 } else { 0 };

    let mut virt_addr = address
        .align_down(if page_size == PageStateChangePageSize::PageSize4KB {
            Size4KiB::SIZE
        } else {
            Size2MiB::SIZE
        })
        .as_u64();

    let page_size = page_size as u8 as u32;

    unsafe {
        asm!("pvalidate",
            inout("rax") virt_addr,
            in("ecx") page_size,
            in("edx") valid
        )
    }

    if virt_addr == 1 {
        Err(PvalidateError::InvalidInput)
    } else if virt_addr == 6 {
        Err(PvalidateError::EntrySizeMismatch)
    } else if virt_addr == 0 {
        Ok(())
    } else {
        panic!("Invalid pvalidate return code {virt_addr}");
    }
}
