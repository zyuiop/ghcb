use core::cmp::min;
use core::fmt::{Display, Formatter};
use static_assertions::const_assert_eq;

const MAX_CPUID_FUNCTIONS: usize = 64;

/// Leaves whose output depends on the subleaf in ECX (Linux' `cpuid_function_is_indexed`).
const fn is_indexed(leaf: u32) -> bool {
    matches!(
        leaf,
        0x4 | 0x7
            | 0xb
            | 0xd
            | 0xf
            | 0x10
            | 0x12
            | 0x14
            | 0x17
            | 0x18
            | 0x1d
            | 0x1e
            | 0x1f
            | 0x24
            | 0x8000_001d
    )
}

#[derive(Debug)]
#[repr(C, align(0x1000))]
pub struct CPUIDPage {
    count: u32,
    _padding: u32,
    _padding2: u64,
    cpuid: [CPUIDFunction; MAX_CPUID_FUNCTIONS],
    _padding3: [u8; 1008],
}

impl Display for CPUIDPage {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        assert!(self.count <= MAX_CPUID_FUNCTIONS as u32);
        for e in 0..(self.count as usize) {
            f.write_str("- ")?;
            Display::fmt(&self.cpuid[e], f)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

impl CPUIDPage {
    pub fn entries(&self) -> usize {
        self.count as usize
    }

    pub fn get_cpuid(&self, eax: u32, ecx: u32, xcr0: u64) -> Option<&CPUIDFunction> {
        if eax & 0x8000_FFFF != eax {
            // Only standard range is checked: 0000_0000 to 0000_FFFF and 8000_0000 to 8000_FFFF
            return None;
        }

        let count = min(self.count as usize, MAX_CPUID_FUNCTIONS);
        for page in self.cpuid.iter().take(count) {
            if page.eax_in == eax && (!is_indexed(eax) || page.ecx_in == ecx) {
                if eax == 0xD {
                    // Check XCR0
                    if page.xcr0_in != xcr0 {
                        continue;
                    }
                }
                return Some(page);
            }
        }

        None
    }
}

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct CPUIDFunction {
    eax_in: u32,
    ecx_in: u32,
    xcr0_in: u64,
    xss_in: u64,
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    _padding: u64,
}

impl Display for CPUIDFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#x}:{:#x} [xcr0={:#x}, xss={:#x}] => eax={:#x}, ebx={:#x}, ecx={:#x}, edx={:#x}",
            self.eax_in,
            self.ecx_in,
            self.xcr0_in,
            self.xss_in,
            self.eax,
            self.ebx,
            self.ecx,
            self.edx
        )
    }
}

const_assert_eq!(size_of::<CPUIDFunction>(), 48);
const_assert_eq!(size_of::<CPUIDPage>(), 4096);
