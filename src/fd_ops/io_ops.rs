use std::io;
use libc::{self, c_void, ssize_t, sockaddr, socklen_t};
use unsafe_cell::{UnsafeRefCell, UnsafeSliceCell};
use error::{ErrCode, READY, ECANCELED, EINTR, EAGAIN, EINPROGRESS,
            last_error, stopped, eof, write_zero};
use io_service::{IoObject, Handler, AsyncResult, IoActor};
use traits::{SockAddr};
use super::{RawFd, AsRawFd, setnonblock, getnonblock};

pub trait AsIoActor : IoObject + AsRawFd + 'static {
    fn as_io_actor(&self) -> &IoActor;
}

pub fn cancel<T>(fd: &T)
    where T: AsIoActor,
{
    let io = fd.io_service();

    for handler in fd.as_io_actor().del_input() {
        io.post(|io| handler(io, ECANCELED));
    }

    for handler in fd.as_io_actor().del_output() {
        io.post(|io| handler(io, ECANCELED));
    }
}

pub fn connect<T, E>(fd: &T, ep: &E) -> io::Result<()>
    where T: AsIoActor,
          E: SockAddr,
{
    while !fd.io_service().stopped() {
        if unsafe { libc::connect(
            fd.as_raw_fd(),
            ep.as_sockaddr() as *const _ as *const sockaddr,
            ep.size() as socklen_t
        ) } == 0 { return Ok(()); }
        let ec = last_error();
        if ec != EINTR {
            return Err(ec.into());
        }
    }
    Err(stopped())
}

pub fn async_connect<T, E, F>(fd: &T, ep: &E, handler: F) -> F::Output
    where T: AsIoActor,
          E: SockAddr,
          F: Handler<()>,
{
    let io = fd.io_service();
    let out = handler.async_result();
    let mode = getnonblock(fd).unwrap();
    setnonblock(fd, true).unwrap();
    if !fd.io_service().stopped() {
        if unsafe { libc::connect(
            fd.as_raw_fd(),
            ep.as_sockaddr() as *const _ as *const sockaddr,
            ep.size() as socklen_t
        ) } == 0 {
            setnonblock(fd, mode).unwrap();
            io.post(move |io| handler.callback(io, Ok(())));
            return out.get(io);
        }

        let ec = last_error();
        if ec == EINPROGRESS {
            let fd_ptr = UnsafeRefCell::new(fd);
            fd.as_io_actor().add_output(handler.wrap(move |io, ec, handler| {
                let fd = unsafe { fd_ptr.as_ref() };
                fd.as_io_actor().next_output();
                setnonblock(fd, mode).unwrap();
                handler.callback(io, match ec {
                    READY => Ok(()),
                    ec => Err(ec.into()),
                });
            }), ec);
            return out.get(io);
        }
        if ec != EINTR {
            setnonblock(fd, mode).unwrap();
            io.post(move |io| handler.callback(io, Err(ec.into())));
            return out.get(io);
        }
    }

    setnonblock(fd, mode).unwrap();
    io.post(move |io| handler.callback(io, Err(stopped())));
    out.get(io)
}


pub fn accept<T, E>(fd: &T, mut ep: E) -> io::Result<(RawFd, E)>
    where T: AsIoActor,
          E: SockAddr,
{
    let mut socklen = ep.capacity() as socklen_t;
    while !fd.io_service().stopped() {
        let acc = unsafe { libc::accept(
            fd.as_raw_fd(),
            ep.as_mut_sockaddr() as *mut _ as *mut sockaddr,
            &mut socklen
        ) };
        if acc >= 0 {
            unsafe { ep.resize(socklen as usize); }
            return Ok((acc, ep));
        }
        let ec = last_error();
        if ec != EINTR {
            return Err(ec.into());
        }
    }
    Err(stopped())
}

fn async_accept_detail<T, E, F>(fd: &T, mut ep: E, handler: F, ec: ErrCode) -> F::Output
    where T: AsIoActor,
          E: SockAddr,
          F: Handler<(RawFd, E)>,
{
    let io = fd.io_service();
    let out = handler.async_result();
    let fd_ptr = UnsafeRefCell::new(fd);
    fd.as_io_actor().add_input(handler.wrap(move |io, ec, handler| {
        let fd = unsafe { fd_ptr.as_ref() };
        match ec {
            READY => {
                let mode = getnonblock(fd).unwrap();
                setnonblock(fd, true).unwrap();

                let mut socklen = ep.capacity() as socklen_t;
                while !io.stopped() {
                    let acc = unsafe { libc::accept(
                        fd.as_raw_fd(),
                        ep.as_mut_sockaddr() as *mut _ as *mut sockaddr,
                        &mut socklen
                    ) };
                    if acc >= 0 {
                        unsafe { ep.resize(socklen as usize); }
                        fd.as_io_actor().next_input();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Ok((acc, ep)));
                        return;
                    }
                    let ec = last_error();
                    if ec == EAGAIN {
                        setnonblock(fd, mode).unwrap();
                        async_accept_detail(fd, ep, handler, ec);
                        return;
                    }
                    if ec != EINTR {
                        fd.as_io_actor().next_input();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Err(ec.into()));
                        return;
                    }
                }
                fd.as_io_actor().next_input();
                setnonblock(fd, mode).unwrap();
                handler.callback(io, Err(stopped()));
            },
            ec => {
                fd.as_io_actor().next_input();
                handler.callback(io, Err(ec.into()));
            },
        }
    }), ec);
    out.get(io)
}

pub fn async_accept<T, E, F>(fd: &T, ep: E, handler: F) -> F::Output
    where T: AsIoActor,
          E: SockAddr,
          F: Handler<(RawFd, E)>,
{
    async_accept_detail(fd, ep, handler, READY)
}


trait Reader : Send + 'static {
    type Output;
    unsafe fn read(&mut self, fd: RawFd, buf: &mut [u8]) -> ssize_t;
    fn ok(self, len: ssize_t) -> Self::Output;
}

fn read_detail<T, R>(fd: &T, buf: &mut [u8], mut reader: R) -> io::Result<R::Output>
    where T: AsIoActor,
          R: Reader,
{
    while !fd.io_service().stopped() {
        let len = unsafe { reader.read(fd.as_raw_fd(), buf) };
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
    Err(stopped())
}

fn async_read_detail<T, R, F>(fd: &T, buf: &mut [u8], mut reader: R, handler: F, ec: ErrCode) -> F::Output
    where T: AsIoActor,
          R: Reader,
          F: Handler<R::Output>,
{
    let io = fd.io_service();
    let out = handler.async_result();
    let fd_ptr = UnsafeRefCell::new(fd);
    let mut buf_ptr = UnsafeSliceCell::new(buf);
    fd.as_io_actor().add_input(handler.wrap(move |io, ec, handler| {
        let fd = unsafe { fd_ptr.as_ref() };
        match ec {
            READY => {
                let buf = unsafe { buf_ptr.as_mut_slice() };
                let mode = getnonblock(fd).unwrap();
                setnonblock(fd, true).unwrap();

                while !io.stopped() {
                    let len = unsafe { reader.read(fd.as_raw_fd(), buf) };
                    if len > 0 {
                        fd.as_io_actor().next_input();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Ok(reader.ok(len)));
                        return;
                    }
                    if len == 0 {
                        fd.as_io_actor().next_input();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Err(eof()));
                        return;
                    }
                    let ec = last_error();
                    if ec == EAGAIN {
                        setnonblock(fd, mode).unwrap();
                        async_read_detail(fd, buf, reader, handler, ec);
                        return;
                    }
                    if ec != EINTR {
                        fd.as_io_actor().next_input();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Err(ec.into()));
                        return;
                    }
                }
                fd.as_io_actor().next_input();
                setnonblock(fd, mode).unwrap();
                handler.callback(io, Err(stopped()));
            },
            ec => {
                fd.as_io_actor().next_input();
                handler.callback(io, Err(ec.into()));
            },
        }
    }), ec);
    out.get(io)
}


struct Read;

impl Reader for Read {
    type Output = usize;

    unsafe fn read(&mut self, fd: RawFd, buf: &mut [u8]) -> ssize_t {
        libc::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len())
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn read<T>(fd: &T, buf: &mut [u8]) -> io::Result<usize>
    where T: AsIoActor,
{
    read_detail(fd, buf, Read)
}

pub fn async_read<T, F>(fd: &T, buf: &mut [u8], handler: F) -> F::Output
    where T: AsIoActor,
          F: Handler<usize>,
{
    async_read_detail(fd, buf, Read, handler, READY)
}


struct Recv { flags: i32 }

impl Reader for Recv {
    type Output = usize;

    unsafe fn read(&mut self, fd: RawFd, buf: &mut [u8]) -> ssize_t {
        libc::recv(fd, buf.as_mut_ptr() as *mut c_void, buf.len(), self.flags)
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn recv<T>(fd: &T, buf: &mut [u8], flags: i32) -> io::Result<usize>
    where T: AsIoActor,
{
    read_detail(fd, buf, Recv { flags: flags })
}

pub fn async_recv<T, F>(fd: &T, buf: &mut [u8], flags: i32, handler: F) -> F::Output
    where T: AsIoActor,
          F: Handler<usize>,
{
    async_read_detail(fd, buf, Recv { flags: flags }, handler, READY)
}


struct RecvFrom<E> { flags: i32, ep: E, socklen: socklen_t }

impl<E: SockAddr + Send> Reader for RecvFrom<E> {
    type Output = (usize, E);

    unsafe fn read(&mut self, fd: RawFd, buf: &mut [u8]) -> libc::ssize_t {
        libc::recvfrom(fd, buf.as_mut_ptr() as *mut c_void,
                       buf.len(), self.flags,
                       self.ep.as_mut_sockaddr() as *mut _ as *mut sockaddr,
                       &mut self.socklen)
    }

    fn ok(mut self, len: ssize_t) -> Self::Output {
        unsafe { self.ep.resize(self.socklen as usize); }
        (len as usize, self.ep)
    }
}

pub fn recvfrom<T, E>(fd: &T, buf: &mut [u8], flags: i32, ep: E) -> io::Result<(usize, E)>
    where T: AsIoActor,
          E: SockAddr,
{
    let socklen = ep.capacity() as socklen_t;
    read_detail(fd, buf, RecvFrom { flags: flags, ep: ep, socklen: socklen })
}

pub fn async_recvfrom<T, E, F>(fd: &T, buf: &mut [u8], flags: i32,   ep: E, handler: F) -> F::Output
    where T: AsIoActor,
          E: SockAddr,
          F: Handler<(usize, E)>,
{
    let socklen = ep.capacity() as socklen_t;
    async_read_detail(fd, buf, RecvFrom { flags: flags, ep: ep, socklen: socklen }, handler, READY)
}



trait Writer : Send + 'static{
    type Output;
    unsafe fn write(&self, fd: RawFd, buf: &[u8]) -> ssize_t;
    fn ok(self, len: ssize_t) -> Self::Output;
}

fn write_detail<T, W>(fd: &T, buf: &[u8], writer: W) -> io::Result<W::Output>
    where T: AsIoActor,
          W: Writer,
{
    while !fd.io_service().stopped() {
        let len = unsafe { writer.write(fd.as_raw_fd(), buf) };
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
    Err(stopped())
}

fn async_write_detail<T, W, F>(fd: &T, buf: &[u8], writer: W, handler: F, ec: ErrCode) -> F::Output
    where T: AsIoActor,
          W: Writer,
          F: Handler<W::Output>,
{
    let io = fd.io_service();
    let out = handler.async_result();
    let fd_ptr = UnsafeRefCell::new(fd);
    let buf_ptr = UnsafeSliceCell::new(buf);
    fd.as_io_actor().add_output(handler.wrap(move |io, ec, handler| {
        let fd = unsafe { fd_ptr.as_ref() };

        match ec {
            READY => {
                let buf = unsafe { buf_ptr.as_slice() };
                let mode = getnonblock(fd).unwrap();
                setnonblock(fd, true).unwrap();

                while !io.stopped() {
                    let len = unsafe { writer.write(fd.as_raw_fd(), buf) };
                    if len > 0 {
                        fd.as_io_actor().next_output();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Ok(writer.ok(len)));
                        return;
                    }
                    if len == 0 {
                        fd.as_io_actor().next_output();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Err(eof()));
                        return;
                    }
                    let ec = last_error();
                    if ec == EAGAIN {
                        setnonblock(fd, mode).unwrap();
                        async_write_detail(fd, buf, writer, handler, ec);
                        return;
                    }
                    if ec != EINTR {
                        fd.as_io_actor().next_output();
                        setnonblock(fd, mode).unwrap();
                        handler.callback(io, Err(ec.into()));
                        return;
                    }
                }
                fd.as_io_actor().next_output();
                setnonblock(fd, mode).unwrap();
                handler.callback(io, Err(stopped()));
            },
            ec => {
                fd.as_io_actor().next_output();
                handler.callback(io, Err(ec.into()));
            },
        }
    }), ec);
    out.get(io)
}


struct Write;

impl Writer for Write {
    type Output = usize;

    unsafe fn write(&self, fd: RawFd, buf: &[u8]) -> ssize_t {
        libc::write(fd, buf.as_ptr() as *const c_void, buf.len())
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn write<T>(fd: &T, buf: &[u8]) -> io::Result<usize>
    where T: AsIoActor,
{
    write_detail(fd, buf, Write)
}

pub fn async_write<T, F>(fd: &T, buf: &[u8], handler: F) -> F::Output
    where T: AsIoActor,
          F: Handler<usize>,
{
    async_write_detail(fd, buf, Write, handler, READY)
}


struct Sent { flags: i32 }

impl Writer for Sent {
    type Output = usize;

    unsafe fn write(&self, fd: RawFd, buf: &[u8]) -> ssize_t {
        libc::send(fd, buf.as_ptr() as *const c_void, buf.len(), self.flags)
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn send<T>(fd: &T, buf: &[u8], flags: i32) -> io::Result<usize>
    where T: AsIoActor,
{
    write_detail(fd, buf, Sent { flags: flags })
}

pub fn async_send<T, F>(fd: &T, buf: &[u8], flags: i32, handler: F) -> F::Output
    where T: AsIoActor,
          F: Handler<usize>,
{
    async_write_detail(fd, buf, Sent { flags: flags }, handler, READY)
}

struct SendTo<E> { flags: i32, ep: E }

impl<E: SockAddr + Send> Writer for SendTo<E> {
    type Output = usize;

    unsafe fn write(&self, fd: RawFd, buf: &[u8]) -> ssize_t {
        libc::sendto(fd, buf.as_ptr() as *const c_void,
                     buf.len(), self.flags,
                     self.ep.as_sockaddr() as *const _ as *const sockaddr,
                     self.ep.size() as socklen_t)
    }

    fn ok(self, len: ssize_t) -> Self::Output {
        len as usize
    }
}

pub fn sendto<T, E>(fd: &T, buf: &[u8], flags: i32, ep: E) -> io::Result<usize>
    where T: AsIoActor,
          E: SockAddr,
{
    write_detail(fd, buf, SendTo { flags: flags, ep: ep })
}

pub fn async_sendto<T, E, F>(fd: &T, buf: &[u8], flags: i32, ep: E, handler: F) -> F::Output
    where T: AsIoActor,
          E: SockAddr,
          F: Handler<usize>,
{
    async_write_detail(fd, buf, SendTo { flags: flags, ep: ep }, handler, READY)
}
