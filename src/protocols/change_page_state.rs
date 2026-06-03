use crate::protocols::GhcbProtocolRequest;
use crate::protocols::change_page_state::PageStateChangePageSize::{PageSize2MB, PageSize4KB};
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::exit_codes::GhcbExitCode;
use crate::structures::ghcb_page::GhcbU64Field;
use bitfield_struct::{bitenum, bitfield};
use x86_64::addr::PhysAddr;
use x86_64::structures::paging::{PageSize, PhysFrame, Size2MiB, Size4KiB};

#[bitfield(u64)]
struct PageStateChangeHeader {
    cur_entry: u16,
    end_entry: u16,
    _reserved: u32,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
#[bitenum]
pub enum PageStateChangeOperation {
    #[fallback]
    PageAssignPrivate = 0x1,
    PageAssignShared = 0x2,
    PageSmash = 0x3,
    PageUnsmash = 0x4,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
#[bitenum]
pub enum PageStateChangePageSize {
    #[fallback]
    PageSize4KB = 0,
    PageSize2MB = 1,
}

#[bitfield(u64)]
pub struct PageStateChangeEntry {
    #[bits(12)]
    pub current_page: u16,
    #[bits(40)]
    pub frame_number: u64,
    #[bits(4, default = PageStateChangeOperation::PageAssignPrivate)]
    pub page_operation: PageStateChangeOperation,
    #[bits(1)]
    pub page_size: PageStateChangePageSize,
    #[bits(7, default = 0)]
    _mbz: u8,
}

impl PageStateChangeEntry {
    pub fn new_for_frame<S: PageSize>(
        physical_frame: PhysFrame<S>,
        operation: PageStateChangeOperation,
    ) -> PageStateChangeEntry {
        let size = match S::SIZE {
            Size4KiB::SIZE => PageSize4KB,
            Size2MiB::SIZE => PageSize2MB,
            other => panic!("unsupported frame size: 0x{other:x}"),
        };

        let gfn: u64 = (physical_frame.start_address().as_u64() >> 12) & 0xff_ffff_ffff; // 40 bits
        Self::new()
            .with_page_size(size)
            .with_page_operation(operation)
            .with_frame_number(gfn)
            .with_current_page(0)
    }

    pub fn physical_address(&self) -> PhysAddr {
        PhysAddr::new(self.frame_number() << 12)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ChangePageStateError {
    /// The page state change request was interrupted.
    /// Parameter is the number of processed entries
    /// Retry the request as is, with no modification
    Interrupted(usize),
    /// The header of the page change structure is invalid
    InvalidHeader,
    /// The change entry at index x is invalid
    InvalidEntry(usize),

    /// Unsmash error for the given entry
    UnsmashError(usize, u32),

    /// The change entry at index x causes an overlap between 2MB and 4KB RMP entries
    RMPOverlap(usize),

    /// Hypervisor encountered an unknown error at given entry. The LSB of the error are provided
    /// in second argument
    UnknownError(usize, u32),
    UnknownErrorCode,
}

impl ChangePageStateError {
    fn with_offset(self, offset: usize) -> Self {
        match self {
            ChangePageStateError::Interrupted(v) => ChangePageStateError::Interrupted(offset + v),
            ChangePageStateError::InvalidEntry(v) => ChangePageStateError::InvalidEntry(offset + v),
            ChangePageStateError::UnsmashError(v, e) => {
                ChangePageStateError::UnsmashError(offset + v, e)
            }
            ChangePageStateError::RMPOverlap(v) => ChangePageStateError::RMPOverlap(offset + v),
            ChangePageStateError::UnknownError(v, e) => {
                ChangePageStateError::UnknownError(offset + v, e)
            }
            ChangePageStateError::InvalidHeader => ChangePageStateError::InvalidHeader,
            ChangePageStateError::UnknownErrorCode => ChangePageStateError::UnknownErrorCode,
        }
    }
}

pub struct ChangePageStateRequest<'a>(&'a [PageStateChangeEntry]);

impl<'a> ChangePageStateRequest<'a> {
    pub fn new(requests: &'a [PageStateChangeEntry]) -> ChangePageStateRequest<'a> {
        ChangePageStateRequest(requests)
    }
}

const MAX_CHANGES_PER_REQUEST: usize = 253;

struct PageStateChangeSharedBuffer {
    header: PageStateChangeHeader,
    changes: [PageStateChangeEntry; MAX_CHANGES_PER_REQUEST],
}

impl ChangePageStateRequest<'_> {
    fn do_execute_request(
        self,
        ghcb: &mut GhcbRequestExecutor,
    ) -> Result<(), ChangePageStateError> {
        if self.0.is_empty() {
            return Ok(());
        }

        assert!(self.0.len() <= MAX_CHANGES_PER_REQUEST);

        let chg_header = PageStateChangeHeader::new().with_end_entry(self.0.len() as u16);

        ghcb.raw().clear();
        ghcb.raw().use_shared_buffer();

        // SAFETY: ensure the array is big enough before transmuting it
        assert_eq!(
            ghcb.raw().shared_buffer_size(),
            size_of::<PageStateChangeSharedBuffer>()
        );
        let shared_buffer = unsafe {
            // SAFETY: we have confirmed the memory is the correct size
            ghcb.raw()
                .shared_buffer_raw_mut()
                .cast::<PageStateChangeSharedBuffer>()
                .as_mut()
                .unwrap()
        };

        shared_buffer.header = chg_header;
        for (index, change) in self.0.iter().enumerate() {
            shared_buffer.changes[index] = change.clone();
        }

        // Execute the request
        ghcb.checked_vmgexit(GhcbExitCode::PageStateChange, 0, 0);

        unsafe {
            core::arch::asm!("mfence", options(preserves_flags, nostack));
        }

        // Read the results (only the header matters)
        // We re-make the pointer so that rust does not optimize and assume the memory did not change
        assert_eq!(
            ghcb.raw().shared_buffer_size(),
            size_of::<PageStateChangeSharedBuffer>()
        );
        let shared_buffer = unsafe {
            // SAFETY: we have confirmed the memory is the correct size
            ghcb.raw()
                .shared_buffer_raw_mut()
                .cast::<PageStateChangeSharedBuffer>()
                .as_mut()
                .unwrap()
        };

        if shared_buffer.header.cur_entry() > shared_buffer.header.end_entry() {
            // Processing was completed successfully
            return Ok(());
        }

        // Interpret the error code.
        let exit2 = ghcb
            .raw()
            .get_field_if_valid(GhcbU64Field::SwExitInfo2)
            .expect("failed to retrieve SWExitInfo2");

        let cur_entry = shared_buffer.header.cur_entry() as usize;

        if exit2 == 0 {
            // Execution was just interrupted, and can be continued later
            // Ideally we'd return `self` as well, but since we did some splitting, we'll let the user reconstruct this if they want...
            // TODO: automatically continue until everything is done?
            return Err(ChangePageStateError::Interrupted(cur_entry));
        };

        let high_bytes = (exit2 >> 32) as u32;
        let low_bytes = (exit2 & 0xffff_ffff) as u32;

        // Error code handling logic
        if high_bytes == 1 {
            if low_bytes == 1 {
                Err(ChangePageStateError::InvalidHeader)
            } else if low_bytes == 2 {
                Err(ChangePageStateError::InvalidEntry(cur_entry))
            } else {
                Err(ChangePageStateError::UnknownErrorCode)
            }
        } else if high_bytes == 2 {
            Err(ChangePageStateError::UnsmashError(cur_entry, low_bytes))
        } else if high_bytes == 3 {
            if low_bytes == 0x1 {
                // cur_entry requested a change that was already processed; ignore and step to next one
                if shared_buffer.header.cur_entry() == shared_buffer.header.end_entry() {
                    Ok(())
                } else {
                    Err(ChangePageStateError::Interrupted(cur_entry + 1))
                }
            } else if low_bytes == 0x2 {
                Err(ChangePageStateError::RMPOverlap(cur_entry))
            } else {
                Err(ChangePageStateError::UnknownErrorCode)
            }
        } else if high_bytes == 0x100 {
            Err(ChangePageStateError::UnknownError(cur_entry, low_bytes))
        } else {
            Err(ChangePageStateError::UnknownErrorCode)
        }
    }
}
impl GhcbProtocolRequest for ChangePageStateRequest<'_> {
    type Response = Result<(), ChangePageStateError>;

    fn execute_request(mut self, ghcb: &mut GhcbRequestExecutor) -> Self::Response {
        // If the request is too big to be sent in one go, split it
        if self.0.len() > MAX_CHANGES_PER_REQUEST {
            for offset in (0..self.0.len()).step_by(MAX_CHANGES_PER_REQUEST) {
                let end = core::cmp::min(self.0.len(), offset + MAX_CHANGES_PER_REQUEST);
                ChangePageStateRequest(&self.0[offset..end])
                    .execute_request(ghcb)
                    .map_err(|err| err.with_offset(offset))?;
            }
            return Ok(());
        }

        let Err(error) = Self(self.0).do_execute_request(ghcb) else {
            return Ok(());
        };

        // Some errors are recoverable
        match error {
            ChangePageStateError::Interrupted(offset) if offset > 0 => {
                // Retry self with offset!
                Self(&self.0[offset..]).execute_request(ghcb)
            }
            err @ ChangePageStateError::RMPOverlap(offset) => {
                // Split the item and retry
                let offending_item = self.0[offset].clone();

                if offending_item.page_size() == PageSize2MB {
                    // We are overlapping with a 4KB page. We shall split this request in 512 sub requests...
                    // We will do this another time!
                    Err(err)
                } else {
                    // We are overlapping with a 2MB page. We shall rewrite this request as a 2MB with offset.
                    let gfn = offending_item.frame_number();
                    let page = gfn & 0x1ff;
                    let gfn = gfn & 0xffff_ffff_ffff_fe00; // Clear last 9 bits

                    let offending_item = offending_item
                        .with_frame_number(gfn)
                        .with_current_page(page as u16);

                    ChangePageStateRequest(&[offending_item]).execute_request(ghcb)?;

                    let rest = &self.0[offset + 1..];
                    if !rest.is_empty() {
                        Self(rest).execute_request(ghcb)?;
                    }
                    Ok(())
                }
            }
            other => Err(other),
        }
    }
}

#[cfg(all(test))]
mod tests {
    use super::PageStateChangeOperation;
    use super::{PageStateChangeEntry, PageStateChangePageSize};

    #[test]
    fn page_state_change_bitfield_is_correct() {
        let offset = 0;
        let size = PageStateChangePageSize::PageSize4KB;
        let operation = PageStateChangeOperation::PageAssignShared;

        let physical_address = 0xc000000000u64;
        let gfn: u64 = (physical_address >> 12) & 0xff_ffff_ffff; // 40 bits

        if size == PageStateChangePageSize::PageSize4KB {
            assert_eq!(offset, 0);
        }

        let entry = PageStateChangeEntry::new()
            .with_page_size(size)
            .with_page_operation(operation)
            .with_frame_number(gfn)
            .with_current_page(offset);

        let size = 0;
        let operation = 2;

        let expected =
            (size & 0x1) << 56 | (operation & 0xf) << 52 | (gfn) << 12 | (offset as u64 & 0xfff);

        assert_eq!(entry.0, expected);
    }
}
