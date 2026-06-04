use crate::protocols::ioio::IoIoPort;
use crate::structures::ChannelManager;
use crate::structures::channel::GhcbRequestExecutor;
use core::hint::spin_loop;
use core::marker::PhantomData;
use embedded_io::{ErrorKind, ErrorType, Read, Write};

const LCR_DLA_ENABLE: u8 = 0b1000_0000;
const LCR_DATA_BITS_8: u8 = 0b0000_0011;

const FIFO_ENABLE: u8 = 0b0000_0001;
const FIFO_CLEAR_RECEIVE: u8 = 0b0000_0010;
const FIFO_CLEAR_TRANSMIT: u8 = 0b0000_0100;
// Wait 14 bytes before triggering an interrupt
const FIFO_SIZE_14BYTES: u8 = 0b1100_0000;

const MCR_DTR_PIN: u8 = 0b0000_0001;
const MCR_RTS_PIN: u8 = 0b0000_0010;
const MCR_OUT2: u8 = 0b0000_1000;

const STS_CAN_SEND: u8 = 0b0010_0000;
const STS_CAN_RECV: u8 = 0b0000_0001;

/// A serial port that uses direct GHCB calls
#[derive(Debug)]
struct SevSerialPortInner {
    data: IoIoPort<u8>,
    interrupt_enable: IoIoPort<u8>,
    fifo_control: IoIoPort<u8>,
    line_control: IoIoPort<u8>,
    modem_control: IoIoPort<u8>,
    line_status: IoIoPort<u8>,
}

struct Config {
    with_interrupts: bool,
}

impl SevSerialPortInner {
    const fn new(base: u16) -> Self {
        Self {
            data: IoIoPort::new(base),
            interrupt_enable: IoIoPort::new(base + 1),
            fifo_control: IoIoPort::new(base + 2),
            line_control: IoIoPort::new(base + 3),
            modem_control: IoIoPort::new(base + 4),
            line_status: IoIoPort::new(base + 5),
        }
    }

    fn init(&self, ghcb: &mut GhcbRequestExecutor, config: &Config) {
        // Disable interrupts
        self.interrupt_enable.write(ghcb, 0x00);

        // Set baud rate divisor to 0x01 (115200 bps)
        self.line_control.write(ghcb, LCR_DLA_ENABLE); // enable DLAB
        self.data.write(ghcb, 0x01); // set low bits
        self.interrupt_enable.write(ghcb, 0x00); // set high bits

        // Disable DLA and set 8 bits of data length
        // Implicitly, stop bits are set to 1 and parity bits to 0
        // See https://wiki.osdev.org/Serial_Ports
        self.line_control.write(ghcb, LCR_DATA_BITS_8);

        // Enable FIFO buffer
        // Allows pushing bytes faster than the receiving end cand process
        self.fifo_control.write(
            ghcb,
            FIFO_SIZE_14BYTES | FIFO_CLEAR_RECEIVE | FIFO_CLEAR_TRANSMIT | FIFO_ENABLE,
        );

        // Configure interrupts and enable
        if config.with_interrupts {
            self.modem_control
                .write(ghcb, MCR_DTR_PIN | MCR_RTS_PIN | MCR_OUT2);
            self.interrupt_enable.write(ghcb, 0x01);
        } else {
            self.modem_control.write(ghcb, MCR_DTR_PIN | MCR_RTS_PIN);
        }
    }

    fn can_send(&self, ghcb: &mut GhcbRequestExecutor) -> bool {
        self.line_status.read(ghcb) & STS_CAN_SEND != 0
    }

    fn can_receive(&self, ghcb: &mut GhcbRequestExecutor) -> bool {
        self.line_status.read(ghcb) & STS_CAN_RECV != 0
    }

    fn write_raw(&mut self, ghcb: &mut GhcbRequestExecutor, b: u8) {
        while !self.can_send(ghcb) {
            spin_loop();
        }

        self.data.write(ghcb, b);
    }

    /// Reads all available bytes into the output buffer and returns the number of bytes read.
    /// Returns immediately if no byte is available
    fn read_all(&mut self, ghcb: &mut GhcbRequestExecutor, output: &mut [u8]) -> usize {
        let mut i = 0usize;
        while self.can_receive(ghcb) && i < output.len() {
            output[i] = self.data.read(ghcb);
            i += 1;
        }
        i
    }
}

/// A serial port backed by a GHCB, that can be used to read or write data
#[derive(Debug)]
pub struct SevSerialPort<C: ChannelManager>(SevSerialPortInner, PhantomData<C>);

impl<C: ChannelManager> SevSerialPort<C> {
    #[inline(always)]
    pub const fn new(port: u16) -> Self {
        Self(SevSerialPortInner::new(port), PhantomData)
    }
}

impl<C: ChannelManager> SevSerialPort<C> {
    pub fn init(&self, enable_interrupts: bool) {
        C::get_channel().with_ghcb(|mut ghcb| {
            self.0.init(
                &mut ghcb,
                &Config {
                    with_interrupts: enable_interrupts,
                },
            );
        })
    }
    pub fn read_ready(&self) -> bool {
        C::get_channel().with_ghcb(|mut ghcb| self.0.can_receive(&mut ghcb))
    }

    /// Reads all available bytes into the output buffer and returns the number of bytes read.
    /// Returns immediately if no byte is available (does not block)
    pub fn read_immediate(&mut self, out: &mut [u8]) -> usize {
        C::get_channel().with_ghcb(|mut ghcb| self.0.read_all(&mut ghcb, out))
    }
}

impl<C: ChannelManager> ErrorType for SevSerialPort<C> {
    type Error = ErrorKind;
}

impl<C: ChannelManager> Write for SevSerialPort<C> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        C::get_channel().with_ghcb(|mut ghcb| {
            for byte in buf {
                self.0.write_raw(&mut ghcb, *byte);
            }
        });

        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<C: ChannelManager> Read for SevSerialPort<C> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.len() == 0 {
            return Ok(0);
        }

        C::get_channel().with_ghcb(|mut ghcb| {
            while !self.0.can_receive(&mut ghcb) {
                spin_loop();
            }

            Ok(self.0.read_all(&mut ghcb, buf))
        })
    }
}

pub struct SevPanicPort<C: ChannelManager> {
    port: SevSerialPortInner,
    init: bool,
    _phantom: PhantomData<C>,
}

impl<C: ChannelManager> SevPanicPort<C> {
    #[inline(always)]
    /// Create a serial port suitable for panic.
    ///
    /// The type used for [C] should return a channel inconditionnally, to ensure we can always log
    /// the panic message.
    ///
    /// ## Safety
    ///
    /// Any usage of this port will cause [GhcbChannel::with_ghcb_force] to be called.
    /// Caller should make sure application will stop after call.
    pub unsafe fn new(port: u16) -> Self {
        Self {
            port: SevSerialPortInner::new(port),
            _phantom: PhantomData,
            init: false,
        }
    }
}

impl<C: ChannelManager> ErrorType for SevPanicPort<C> {
    type Error = ErrorKind;
}

impl<C: ChannelManager> Write for SevPanicPort<C> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        unsafe {
            C::get_channel().with_ghcb_force(|mut ghcb| {
                if !self.init {
                    self.port.init(
                        &mut ghcb,
                        &Config {
                            with_interrupts: false,
                        },
                    );
                    self.init = true;
                }

                for byte in buf {
                    self.port.write_raw(&mut ghcb, *byte);
                }
            })
        };

        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
