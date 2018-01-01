use prelude::{Protocol, SockAddr};
use ffi::{self, socklen_t};
use unsafe_cell::{UnsafeRefCell, UnsafeSliceCell};
use error::{ErrCode, READY, EINTR, EAGAIN, EWOULDBLOCK, ECANCELED, last_error, eof};
use core::{IoContext, ThreadIoContext, workplace};
use async::{Receiver, Handler, WrappedHandler, Operation};
use reactive_io::{AsyncInput, getnonblock, setnonblock};

use std::io;
use libc::ssize_t;

trait Reader : Send + 'static {
    type Output;

    fn read<T>(&mut self, soc: &T, buf: &mut [u8]) -> ssize_t
        where T: AsyncInput;

    fn ok(&self, len: ssize_t) -> Self::Output;
}

struct Read;

impl Reader for Read {
    type Output = usize;

    fn read<T>(&mut self, soc: &T, buf: &mut [u8]) -> ssize_t
        where T: AsyncInput,
    {
        unsafe { ffi::read(soc, buf) }
    }

    fn ok(&self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

struct Recv { flags: i32 }

impl Reader for Recv {
    type Output = usize;

    fn read<T>(&mut self, soc: &T, buf: &mut [u8]) -> ssize_t
        where T: AsyncInput,
    {
        unsafe { ffi::recv(soc, buf, self.flags) }
    }

    fn ok(&self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

struct RecvFrom<E> { flags: i32, ep: E, socklen: socklen_t }

impl<E: SockAddr> Reader for RecvFrom<E> {
    type Output = (usize, E);

    fn read<T>(&mut self, soc: &T, buf: &mut [u8]) -> ssize_t
        where T: AsyncInput,
    {
        unsafe { ffi::recvfrom(soc, buf, self.flags, &mut self.ep, &mut self.socklen) }
    }

    fn ok(&self, len: ssize_t) -> Self::Output {
        let mut ep = self.ep.clone();
        unsafe { ep.resize(self.socklen as usize) };
        (len as usize, ep)
    }
}

fn read_detail<T, R>(soc: &T, buf: &mut [u8], mut reader: R) -> io::Result<R::Output>
    where T: AsyncInput,
          R: Reader,
{
    while !soc.as_ctx().stopped() {
        let len = reader.read(soc, buf);
        if len > 0 {
            return Ok(reader.ok(len));
        }
        if len == 0 {
            return Err(eof());
        }
        let ec = last_error();
        if ec != EINTR {
            return Err(ec.into());
        }
    }
    Err(ECANCELED.into())
}

pub fn read<T>(soc: &T, buf: &mut [u8]) -> io::Result<usize>
    where T: AsyncInput,
{
    read_detail(soc, buf, Read)
}

pub fn recv<T>(soc: &T, buf: &mut [u8], flags: i32) -> io::Result<usize>
    where T: AsyncInput,
{
    read_detail(soc, buf, Recv { flags: flags })
}

pub fn recvfrom<T, P>(soc: &T, buf: &mut [u8], flags: i32, pro: &P) -> io::Result<(usize, P::Endpoint)>
    where T: AsyncInput,
          P: Protocol,
{
    let ep = unsafe { pro.uninitialized() };
    let socklen = ep.capacity() as socklen_t;
    read_detail(soc, buf, RecvFrom { flags: flags, ep: ep, socklen: socklen })
}

struct ReadHandler<T, R> {
    soc: UnsafeRefCell<T>,
    buf: UnsafeSliceCell<u8>,
    reader: R,
}

impl<T, R> WrappedHandler<R::Output, io::Error> for ReadHandler<T, R>
    where T: AsyncInput,
          R: Reader,
{
    fn perform(&mut self, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode, op: Operation<R::Output, io::Error, Self>) {
        let soc = unsafe { self.soc.as_ref() };
        match ec {
            READY => {
                let mode = getnonblock(soc).unwrap();
                setnonblock(soc, true).unwrap();
                while !ctx.stopped() {
                    let buf = unsafe { self.buf.as_mut_slice() };
                    let len = self.reader.read(soc, buf);
                    if len > 0 {
                        setnonblock(soc, mode).unwrap();
                        soc.next_op(this);
                        op.send(ctx, Ok(self.reader.ok(len)));
                        return;
                    }
                    if len == 0 {
                        setnonblock(soc, mode).unwrap();
                        soc.next_op(this);
                        op.send(ctx, Err(eof()));
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

fn async_read_detail<T, F, R>(soc: &T, buf: &mut [u8], handler: F, reader: R) -> F::Output
    where T: AsyncInput,
          F: Handler<R::Output, io::Error>,
          R: Reader,
{
    let (op, res) = handler.channel(ReadHandler {
        soc: UnsafeRefCell::new(soc),
        buf: UnsafeSliceCell::new(buf),
        reader: reader
    });
    workplace(soc.as_ctx(), |this| soc.add_op(this, op, READY));
    res.recv(soc.as_ctx())
}

pub fn async_read<T, F>(soc: &T, buf: &mut [u8], handler: F) -> F::Output
    where T: AsyncInput,
          F: Handler<usize, io::Error>
{
    async_read_detail(soc, buf, handler, Read)
}

pub fn async_recv<T, F>(soc: &T, buf: &mut [u8], flags: i32, handler: F) -> F::Output
    where T: AsyncInput,
          F: Handler<usize, io::Error>,
{
    async_read_detail(soc, buf, handler, Recv { flags: flags })
}

pub fn async_recvfrom<T, E, F>(soc: &T, buf: &mut [u8], flags: i32, ep: E, handler: F) -> F::Output
    where T: AsyncInput,
          E: SockAddr,
          F: Handler<(usize, E), io::Error>,
{
    let socklen = ep.capacity() as socklen_t;
    async_read_detail(soc, buf, handler, RecvFrom { flags: flags, ep: ep, socklen: socklen })
}
