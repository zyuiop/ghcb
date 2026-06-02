use bitfield_struct::bitfield;
use crate::protocols::GhcbProtocolRequest;
use crate::structures::channel::GhcbRequestExecutor;
use crate::structures::ChannelManager;
use crate::structures::exit_codes::GhcbExitCode;
use crate::structures::ghcb_page::GhcbU64Field;

pub struct IoIoRequest<'a> {
    io_port: u16,
    segment_number: u8,
    operation: IoIoOperation<'a>,
}

impl<'a> IoIoRequest<'a> {
    pub fn new(io_port: u16, operation: IoIoOperation<'a>) -> Self {
        Self {
            io_port, operation, segment_number: 0,
        }
    }

    pub fn with_segment(self, segment: u8) -> Self {
        Self {
            segment_number: segment,
            ..self
        }
    }
}


#[inline]
pub fn outb<T: ChannelManager>(port: u16, val: u8) {
    IoIoRequest::new(port, IoIoOperation::ByteOut(val))
        .execute::<T>()
}

#[inline]
pub fn out_u32<T: ChannelManager>(port: u16, val: u32) {
    IoIoRequest::new(port, IoIoOperation::DblWordOut(val))
        .execute::<T>()
}

#[inline]
pub fn in_u32<T: ChannelManager>(port: u16) -> u32 {
    let mut ret: u32 = 0;
    IoIoRequest::new(port, IoIoOperation::DblWordIn(&mut ret))
        .execute::<T>();
    ret
}

#[inline]
pub fn inb<T: ChannelManager>(port: u16) -> u8 {
    let mut ret: u8 = 0;
    IoIoRequest::new(port, IoIoOperation::ByteIn(&mut ret))
        .execute::<T>();
    ret
}

impl GhcbProtocolRequest for IoIoRequest<'_> {
    type Response = ();

    fn execute_request(mut self, ghcb: &mut GhcbRequestExecutor) -> Self::Response {
        ghcb.raw().clear();

        let mut flags = IoIoExitFlags::ADDR_64B;
        let mut rep_count = 0;

        match &mut self.operation {
            IoIoOperation::StringOut(str) => {
                if str.len() > ghcb.raw().shared_buffer_size() {
                    // Split current request...
                    let (str, rest) = str.split_at(ghcb.raw().shared_buffer_size());

                    // Execute first part
                    IoIoRequest {
                        io_port: self.io_port,
                        segment_number: self.segment_number,
                        operation: IoIoOperation::StringOut(str)
                    }.execute_request(ghcb);

                    // And second part
                    return IoIoRequest {
                        io_port: self.io_port,
                        segment_number: self.segment_number,
                        operation: IoIoOperation::StringOut(rest)
                    }.execute_request(ghcb);
                }

                flags.insert(IoIoExitFlags::STRING);
                flags.insert(IoIoExitFlags::REPEAT);
                ghcb.raw().copy_to_shared_buffer(str);
                rep_count = str.len();
            }
            IoIoOperation::StringIn(str) => {
                if str.len() > ghcb.raw().shared_buffer_size() {
                    // Split current request...
                    let (str, rest) = str.split_at_mut(ghcb.raw().shared_buffer_size());

                    // Execute first part
                    IoIoRequest {
                        io_port: self.io_port,
                        segment_number: self.segment_number,
                        operation: IoIoOperation::StringIn(str)
                    }.execute_request(ghcb);

                    // And second part
                    return IoIoRequest {
                        io_port: self.io_port,
                        segment_number: self.segment_number,
                        operation: IoIoOperation::StringIn(rest)
                    }.execute_request(ghcb);
                }

                flags.insert(IoIoExitFlags::STRING);
                flags.insert(IoIoExitFlags::IS_INPUT);
                flags.insert(IoIoExitFlags::REPEAT);
                rep_count = str.len();
            }
            IoIoOperation::ByteOut(v) => {
                flags.insert(IoIoExitFlags::DATA_8B);
                ghcb.raw().set_field(GhcbU64Field::Rax, *v as u64);
            }
            IoIoOperation::WordOut(v) => {
                flags.insert(IoIoExitFlags::DATA_16B);
                ghcb.raw().set_field(GhcbU64Field::Rax, *v as u64);
            }
            IoIoOperation::DblWordOut(v) => {
                flags.insert(IoIoExitFlags::DATA_32B);
                ghcb.raw().set_field(GhcbU64Field::Rax, *v as u64);
            }
            IoIoOperation::ByteIn(_) => {
                flags.insert(IoIoExitFlags::DATA_8B);
                flags.insert(IoIoExitFlags::IS_INPUT);
            }
            IoIoOperation::WordIn(_) => {
                flags.insert(IoIoExitFlags::DATA_16B);
                flags.insert(IoIoExitFlags::IS_INPUT);
            }
            IoIoOperation::DblWordIn(_) => {
                flags.insert(IoIoExitFlags::DATA_32B);
                flags.insert(IoIoExitFlags::IS_INPUT);
            }
        }

        let exit1 = IoIoExitData::new()
            .with_flags(flags.bits())
            .with_port(self.io_port)
            .with_segment_number(self.segment_number)
            .into_bits() as u64;

        ghcb.checked_vmgexit(GhcbExitCode::IoIoProtocol, exit1, rep_count as u64);

        match self.operation {
            IoIoOperation::ByteIn(v) => {
                let rax = ghcb.raw().get_field_if_valid(GhcbU64Field::Rax).unwrap();
                *v = (rax & 0xff) as u8;
            }
            IoIoOperation::WordIn(v) => {
                let rax = ghcb.raw().get_field_if_valid(GhcbU64Field::Rax).unwrap();
                *v = (rax & 0xffff) as u16;
            }
            IoIoOperation::DblWordIn(v) => {
                let rax = ghcb.raw().get_field_if_valid(GhcbU64Field::Rax).unwrap();
                *v = (rax & 0xffff_ffff) as u32;
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub enum IoIoOperation<'a> {
    /// Write a string to the IO port
    StringOut(&'a [u8]),
    /// Read a string from the IO port
    StringIn(&'a mut [u8]),

    /// Write a single byte to the IO port
    ByteOut(u8),
    /// Write a single word to the IO port
    WordOut(u16),
    /// Write a double word to the IO port
    DblWordOut(u32),

    /// Read a single byte from the IO port
    ByteIn(&'a mut u8),
    /// Read a single word from the IO port
    WordIn(&'a mut u16),
    /// Read a double word from the IO port
    DblWordIn(&'a mut u32),
}

bitflags! {
	#[derive(Debug, Copy, Clone, Default)]
	struct IoIoExitFlags: u16 {
		const IS_INPUT = 1 << 0; // Access Type, set to 1 to indicate input

		const STRING = 1 << 2;
		const REPEAT = 1 << 3;

		const DATA_8B = 1 << 4;
		const DATA_16B = 1 << 5;
		const DATA_32B = 1 << 6;

		const ADDR_16B = 1 << 7;
		const ADDR_32B = 1 << 8;
		const ADDR_64B = 1 << 9;
	}
}

#[bitfield(u32)]
struct IoIoExitData {
    #[bits(10)]
    flags: u16,
    #[bits(3)]
    segment_number: u8,
    #[bits(3)]
    _reserved: u8,
    port: u16,
}