use ffi::{RawFd, AsRawFd, SystemError, INVALID_ARGUMENT};
use core::{AsIoContext, IoContext, ThreadIoContext, Perform, SocketImpl};
use handler::{Handler, AsyncReadOp, AsyncWriteOp};
use read_ops::{Read, async_read_op, blocking_read_op, nonblocking_read_op};
use write_ops::{Write, async_write_op, blocking_write_op, nonblocking_write_op};
use stream::Stream;

use std::io;
use std::ffi::CString;
use libc::{self, O_RDWR, O_NOCTTY, O_NDELAY, O_NONBLOCK, O_CLOEXEC};
use termios::{Termios, tcsendbreak};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use self::linux::setup_serial;
#[cfg(target_os = "linux")]
pub use self::linux::{BaudRate, Parity, CSize, FlowControl, StopBits};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use self::macos::setup_serial;
#[cfg(target_os = "macos")]
pub use self::macos::{BaudRate, Parity, CSize, FlowControl, StopBits};

pub trait SerialPortOption: Sized {
    fn load(target: &SerialPort) -> Self;

    fn store(self, target: &mut SerialPort) -> io::Result<()>;
}

pub struct SerialPort {
    pimpl: Box<SocketImpl<Termios>>,
}

impl SerialPort {
    pub fn new(ctx: &IoContext, device: &str) -> io::Result<Self> {
        let fd = match CString::new(device) {
            Ok(device) => {
                let ptr = device.as_bytes_with_nul().as_ptr() as *const i8;
                match unsafe {
                    libc::open(ptr, O_RDWR | O_NOCTTY | O_NDELAY | O_NONBLOCK | O_CLOEXEC)
                } {
                    -1 => return Err(SystemError::last_error().into()),
                    fd => fd,
                }
            }
            _ => return Err(INVALID_ARGUMENT.into()),
        };
        Ok(SerialPort {
            pimpl: SocketImpl::new(ctx, fd, setup_serial(fd)?),
        })
    }

    pub fn cancel(&self) {
        self.pimpl.cancel()
    }

    pub fn get_option<C>(&self) -> C
    where
        C: SerialPortOption,
    {
        C::load(self)
    }

    pub fn nonblocking_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        nonblocking_read_op(self, buf, Read::new())
    }

    pub fn nonblocking_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        nonblocking_write_op(self, buf, Write::new())
    }

    pub fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        blocking_read_op(self, buf, self.pimpl.get_read_timeout(), Read::new())
    }

    pub fn send_break(&self) -> io::Result<()> {
        tcsendbreak(self.as_raw_fd(), 0)
    }

    pub fn set_option<C>(&mut self, cmd: C) -> io::Result<()>
    where
        C: SerialPortOption,
    {
        cmd.store(self)
    }

    pub fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        blocking_write_op(self, buf, self.pimpl.get_write_timeout(), Write::new())
    }
}

unsafe impl Send for SerialPort {}

unsafe impl AsIoContext for SerialPort {
    fn as_ctx(&self) -> &IoContext {
        self.pimpl.as_ctx()
    }
}

impl AsRawFd for SerialPort {
    fn as_raw_fd(&self) -> RawFd {
        self.pimpl.as_raw_fd()
    }
}

impl io::Read for SerialPort {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_some(buf)
    }
}

impl io::Write for SerialPort {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_some(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Stream for SerialPort {
    type Error = io::Error;

    fn async_read_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>,
    {
        async_read_op(self, buf, handler, Read::new())
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>,
    {
        async_write_op(self, buf, handler, Write::new())
    }
}

impl AsyncReadOp for SerialPort {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.pimpl.add_read_op(this, op, err)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.pimpl.next_read_op(this)
    }
}

impl AsyncWriteOp for SerialPort {
    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.pimpl.add_write_op(this, op, err)
    }

    fn next_write_op(&self, this: &mut ThreadIoContext) {
        self.pimpl.next_write_op(this)
    }
}

#[test]
#[ignore]
fn test_serial_port() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    let mut serial_port = SerialPort::new(ctx, "/dev/ttyS0").unwrap();
    serial_port.set_option(BaudRate::B9600).unwrap();
}
