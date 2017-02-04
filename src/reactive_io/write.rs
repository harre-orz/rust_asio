use prelude::SockAddr;
use ffi;
use unsafe_cell::{UnsafeRefCell, UnsafeSliceCell};
use error::{READY, EINTR, EAGAIN, EWOULDBLOCK, ECANCELED,
            ErrCode, last_error, write_zero};
use core::{IoContext, ThreadIoContext, workplace};
use async::{Receiver, Handler, WrappedHandler, Operation};
use reactive_io::{AsyncOutput, getnonblock, setnonblock};

use std::io;
use libc::ssize_t;

trait Writer : Send + 'static {
    type Output;

    fn write<T>(&self, soc: &T, buf: &[u8]) -> ssize_t
        where T: AsyncOutput;

    fn ok(&self, len: ssize_t) -> Self::Output;
}

struct Write;

impl Writer for Write {
    type Output = usize;

    fn write<T>(&self, soc: &T, buf: &[u8]) -> ssize_t
        where T: AsyncOutput
    {
        unsafe { ffi::write(soc, buf) }
    }

    fn ok(&self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

struct Sent { flags: i32 }

impl Writer for Sent {
    type Output = usize;

    fn write<T>(&self, soc: &T, buf: &[u8]) -> ssize_t
        where T: AsyncOutput
    {
        unsafe { ffi::send(soc, buf, self.flags) }
    }

    fn ok(&self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

struct SendTo<E> { flags: i32, ep: E }

impl<E: SockAddr> Writer for SendTo<E> {
    type Output = usize;

    fn write<T>(&self, soc: &T, buf: &[u8]) -> ssize_t
        where T: AsyncOutput,
    {
        unsafe { ffi::sendto(soc, buf, self.flags, &self.ep) }
    }

    fn ok(&self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

fn write_detail<T, W>(soc: &T, buf: &[u8], writer: W) -> io::Result<W::Output>
    where T: AsyncOutput,
          W: Writer,
{
    while !soc.as_ctx().stopped() {
        let len = writer.write(soc, buf);
        if len > 0 {
            return Ok(writer.ok(len));
        }
        if len == 0 {
            return Err(write_zero());
        }
        let ec = last_error();
        if ec != EINTR {
            return Err(ec.into());
        }
    }
    Err(ECANCELED.into())
}

pub fn write<T>(soc: &T, buf: &[u8]) -> io::Result<usize>
    where T: AsyncOutput,
{
    write_detail(soc, buf, Write)
}

pub fn send<T>(soc: &T, buf: &[u8], flags: i32) -> io::Result<usize>
    where T: AsyncOutput,
{
    write_detail(soc, buf, Sent { flags: flags })
}

pub fn sendto<T, E>(soc: &T, buf: &[u8], flags: i32, ep: E) -> io::Result<usize>
    where T: AsyncOutput,
          E: SockAddr,
{
    write_detail(soc, buf, SendTo { flags: flags, ep: ep })
}

struct WriteHandler<T, W> {
    soc: UnsafeRefCell<T>,
    buf: UnsafeSliceCell<u8>,
    writer: W,
}

impl<T, W> WrappedHandler<W::Output, io::Error> for WriteHandler<T, W>
    where T: AsyncOutput,
          W: Writer,
{
    fn perform(&mut self, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode, op: Operation<W::Output, io::Error, Self>) {
        let soc = unsafe { self.soc.as_ref() };
        match ec {
            READY => {
                let mode = getnonblock(soc).unwrap();
                setnonblock(soc, true).unwrap();

                while !ctx.stopped() {
                    let buf = unsafe { self.buf.as_slice() };
                    let len = self.writer.write(soc, buf);
                    if len > 0 {
                        setnonblock(soc, mode).unwrap();
                        soc.next_op(this);
                        op.send(ctx, Ok(self.writer.ok(len)));
                        return;
                    }
                    if len == 0 {
                        setnonblock(soc, mode).unwrap();
                        soc.next_op(this);
                        op.send(ctx, Err(write_zero()));
                        return;
                    }
                    let ec = last_error();
                    if ec == EAGAIN || ec == EWOULDBLOCK {
                        setnonblock(soc, mode).unwrap();
                        soc.add_op(this, op, ec);
                        return;
                    }
                    if ec != EINTR {
                        setnonblock(soc, mode).unwrap();
                        soc.next_op(this);
                        op.send(ctx, Err(ec.into()));
                        return;
                    }
                }
                setnonblock(soc, mode).unwrap();
                soc.next_op(this);
                op.send(ctx, Err(ECANCELED.into()));
            },
            ec => {
                soc.next_op(this);
                op.send(ctx, Err(ec.into()));
            },
        }
    }
}

fn async_write_detail<T, F, W>(soc: &T, buf: &[u8], handler: F, writer: W) -> F::Output
    where T: AsyncOutput,
          F: Handler<W::Output, io::Error>,
          W: Writer,
{
    let (op, res) = handler.channel(WriteHandler {
        soc: UnsafeRefCell::new(soc),
        buf: UnsafeSliceCell::new(buf),
        writer: writer,
    });
    workplace(soc.as_ctx(), |this| soc.add_op(this, op, READY));
    res.recv(soc.as_ctx())
}

pub fn async_write<T, F>(soc: &T, buf: &[u8], handler: F) -> F::Output
    where T: AsyncOutput,
          F: Handler<usize, io::Error>,
{
    async_write_detail(soc, buf, handler, Write)
}

pub fn async_send<T, F>(soc: &T, buf: &[u8], flags: i32, handler: F) -> F::Output
    where T: AsyncOutput,
          F: Handler<usize, io::Error>,
{
    async_write_detail(soc, buf, handler, Sent { flags: flags })
}

pub fn async_sendto<T, E, F>(soc: &T, buf: &[u8], flags: i32, ep: E, handler: F) -> F::Output
    where T: AsyncOutput,
          E: SockAddr,
          F: Handler<usize, io::Error>,
{
    async_write_detail(soc, buf, handler, SendTo { flags: flags, ep: ep })
}
