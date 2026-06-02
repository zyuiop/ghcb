use core::cmp::min;
use static_assertions::const_assert_eq;

const MAX_CPUID_FUNCTIONS: usize = 64;

pub struct CPUIDPage {
    count: u32,
    _padding: u32,
    _padding2: u64,
    cpuid: [CPUIDFunction; MAX_CPUID_FUNCTIONS],
    _padding3: [u8; 1008]
}

impl CPUIDPage {
    pub fn get_cpuid(&self, eax: u32, ecx: u32, xcr0: u64) -> Option<& CPUIDFunction> {
        if eax & 0x8000_FFFF != eax {
            // Only standard range is checked: 0000_0000 to 0000_FFFF and 8000_0000 to 8000_FFFF
            return None;
        }

        let count = min(self.count as usize, MAX_CPUID_FUNCTIONS);
        for page in self.cpuid.iter().take(count) {
            if page.eax_in == eax && page.ecx_in == ecx {
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

pub struct CPUIDFunction {
    eax_in: u32,
    ecx_in: u32,
    xcr0_in: u64,
    xss_in: u64,
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    _padding: u64
}

const_assert_eq!(size_of::<CPUIDFunction>(), 48);
const_assert_eq!(size_of::<CPUIDPage>(), 4096);

