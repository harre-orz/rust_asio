use ffi::{RawFd, AsRawFd, SystemError, Timeout, INVALID_ARGUMENT};
use core::{AsIoContext, IoContext, ThreadIoContext, Perform, InnerSocket};
use ops::*;
use streams::Stream;
use handler::Handler;

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


pub trait SerialPortOption : Sized {
    fn load(target: &SerialPort) -> Self;

    fn store(self, target: &mut SerialPort) -> io::Result<()>;
}

pub struct SerialPort {
    inner: Box<InnerSocket<Termios>>,
}

impl SerialPort {
    pub fn new(ctx: &IoContext, device: &str) -> io::Result<Self> {
        let fd = match CString::new(device) {
            Ok(device) => {
                let ptr = device.as_bytes_with_nul().as_ptr() as *const i8;
                match unsafe { libc::open(ptr, O_RDWR | O_NOCTTY | O_NDELAY | O_NONBLOCK | O_CLOEXEC) } {
                    -1 => return Err(SystemError::last_error().into()),
                    fd => fd,
                }
            },
            _ => return Err(INVALID_ARGUMENT.into()),
        };
        Ok(SerialPort {
            inner: InnerSocket::new(ctx, fd, setup_serial(fd)?)
        })
    }

    pub fn cancel(&self) {
        self.inner.cancel()
    }

    pub fn get_option<C>(&self) -> C
        where C: SerialPortOption,
    {
        C::load(self)
    }

    pub fn nonblocking_read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        nonblocking_read(self, buf)
    }

    pub fn nonblocking_write_some(&self, buf: &[u8]) -> io::Result<usize> {
        nonblocking_write(self, buf)
    }

    pub fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        read_timeout(self, buf, &Timeout::default())
    }

    pub fn send_break(&self) -> io::Result<()> {
        tcsendbreak(self.as_raw_fd(), 0)
    }

    pub fn set_option<C>(&mut self, cmd: C) -> io::Result<()>
        where C: SerialPortOption,
    {
        cmd.store(self)
    }

    pub fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        write_timeout(self, buf, &Timeout::default())
    }
}

unsafe impl Send for SerialPort {}

unsafe impl AsIoContext for SerialPort {
    fn as_ctx(&self) -> &IoContext {
        self.inner.as_ctx()
    }
}

impl AsRawFd for SerialPort {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl io::Read for SerialPort {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_some(buf)
    }
}

impl AsyncSocketOp for SerialPort {
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.inner.add_read_op(this, op, err)
    }

    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.inner.add_write_op(this, op, err)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.inner.next_read_op(this)
    }

    fn next_write_op(&self, this: &mut ThreadIoContext) {
        self.inner.next_write_op(this)
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
        async_read(self, buf, handler)
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>,
    {
        async_write(self, buf, handler)
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
