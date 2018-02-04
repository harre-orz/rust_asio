use ffi::{Handle, AsHandle};
use error::{invalid_argument};
use core::{IoContext, AsIoContext, IoEvent};
use async::{Handler};
use nio::{AsIoEvent, cancel, read, write, async_read, async_write};
use streams::Stream;
use serial_port::{SerialPortOption};
#[cfg(target_os = "linux")] use super::linux::setup;
#[cfg(target_os = "macos")] use super::macos::setup;

use std::io;
use std::ffi::CString;
use libc::{self, O_RDWR, O_NOCTTY, O_NDELAY, O_NONBLOCK, O_CLOEXEC};
use termios::{Termios, tcsendbreak};

pub trait AsTermios {
    fn as_ios(&self) -> &Termios;
    fn as_mut_ios(&mut self) -> &mut Termios;
}

pub struct SerialPort {
    ios: Termios,
    ev: IoEvent,
}

impl SerialPort {
    pub fn new<T>(ctx: &IoContext, device: T) -> io::Result<SerialPort>
        where T: AsRef<str>
    {
        let fd = match CString::new(device.as_ref()) {
            Ok(device) => libc_try!(libc::open(
                device.as_bytes_with_nul().as_ptr() as *const i8,
                )
            ),
            _ => return Err(invalid_argument()),
        };
        let ev = IoEvent::new(fd, ctx);  // fd は素早く IoEvent に指定させる
        Ok(SerialPort {
            ios: try!(setup(fd)),
            ev: ev,
        })
    }

    pub fn cancel(&self) {
        cancel(self)
    }

    pub fn get_option<C>(&self) -> C
        where C: SerialPortOption,
    {
        C::load(self)
    }

    pub fn send_break(&self) -> io::Result<()> {
        tcsendbreak(self.as_handle(), 0)
    }

    pub fn set_option<C>(&mut self, cmd: C) -> io::Result<()>
        where C: SerialPortOption,
    {
        cmd.store(self)
    }
}

impl AsHandle for SerialPort {
    fn as_handle(&self) -> Handle {
        self.ev.as_handle()
    }
}

unsafe impl Send for SerialPort { }

unsafe impl AsIoContext for SerialPort {
    fn as_ctx(&self) -> &IoContext {
        self.ev.as_ctx()
    }
}

impl Stream<io::Error> for SerialPort {
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        async_read(self, buf, handler)
    }

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        async_write(self, buf, handler)
    }

    fn read_some(&self, buf: &mut [u8]) -> io::Result<usize> {
        read(self, buf)
    }

    fn write_some(&self, buf: &[u8]) -> io::Result<usize> {
        write(self, buf)
    }
}

impl AsIoEvent for SerialPort {
    fn as_ev(&self) -> &IoEvent {
        &self.ev
    }
}

impl AsTermios for SerialPort {
    fn as_ios(&self) -> &Termios {
        &self.ios
    }

    fn as_mut_ios(&mut self) -> &mut Termios {
        &mut self.ios
    }
}
