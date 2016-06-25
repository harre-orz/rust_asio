use std::io;
use std::slice;
use {UnsafeThreadableCell, IoObject, IoService, Strand};
use backbone::{Expiry, HandlerResult, EpollIoActor, TimerActor};
use socket::{Protocol, Endpoint, NonBlocking, StreamBuf, MatchCondition};
use libc;
use ops::*;

pub trait AsIoActor {
    fn as_io_actor(&self) -> &EpollIoActor;
}

pub trait AsTimerActor {
    fn as_timer_actor(&self) -> &TimerActor;
}

enum AsyncResult<T> {
    Ok(T),
    Err(io::Error),
    WouldBlock,
}

fn operation_canceled() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Operation canceled")
}

fn connection_refused() -> io::Error {
    io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused")
}

fn connect_with_nonblock<S, E>(soc: &S, ep: &E) -> AsyncResult<()>
    where S: AsRawFd + NonBlocking,
          E: AsRawSockAddr,
{
    if let Err(err) = soc.native_set_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::connect(soc.as_raw_fd(), ep.as_raw_sockaddr(), ep.raw_socklen()) } {
        0 => AsyncResult::Ok(()),
        _ => {
            let err = errno();
            if err == libc::EINPROGRESS {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
    }
}

pub fn connect_async<S, P, E, F, O>(soc: &S, ep: &E, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking + 'static,
          P: Protocol + Send + 'static,
          E: AsRawSockAddr + Endpoint<P> + 'static,
          F: FnOnce(Strand<O>, io::Result<()>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    if let Some(callback) = soc.as_io_actor().unset_out(io) {
        io.0.task.post(obj.id(), Box::new(move |io| callback(io, HandlerResult::Canceled)));
    }
    match connect_with_nonblock(soc, ep) {
        AsyncResult::Err(err) => {
            io.post_strand(move |obj| callback(obj, Err(err)), obj);
        },
        AsyncResult::Ok(_) => {
            io.post_strand(move |obj| callback(obj, Ok(())), obj);
        },
        AsyncResult::WouldBlock => {
            let obj_ = obj.obj.clone();
            let soc_ = UnsafeThreadableCell::new(soc as *const S);
            let pro_ = ep.protocol();
            soc.as_io_actor().set_out(io, obj.id(), Box::new(move |io: *const IoService, res| {
                let obj = Strand { io: unsafe { &*io }, obj: obj_ };
                let soc = unsafe { &**soc_ };
                match res {
                    HandlerResult::Ready => callback(obj, Ok(())),
                    HandlerResult::Canceled => {
                        if let Ok(fd) = socket(pro_) {
                            soc.as_io_actor().reopen(fd);
                        }
                        callback(obj, Err(operation_canceled()))
                    },
                    HandlerResult::Errored => {
                        if let Ok(fd) = socket(pro_) {
                            soc.as_io_actor().reopen(fd);
                        }
                        callback(obj, Err(connection_refused()))
                    },
                }
            }));
        },
    }
}

pub fn connect_syncd<S, E>(soc: &S, ep: &E, io: &IoService) -> io::Result<()>
    where S: AsRawFd + AsIoActor + NonBlocking,
          E: AsRawSockAddr,
{
    if let Some(callback) = soc.as_io_actor().unset_out(io) {
        callback(io, HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    connect(soc, ep)
}

fn accept_with_nonblock<S, E>(soc: &S, ep: &mut E) -> AsyncResult<(RawFd, E)>
    where S: AsRawFd + NonBlocking,
          E: AsRawSockAddr + Clone,
{
    if let Err(err) = setnonblock(soc, true) {
        return AsyncResult::Err(err);
    }
    let mut socklen = ep.raw_socklen();
    match unsafe { libc::accept(soc.as_raw_fd(), ep.as_mut_raw_sockaddr(), &mut socklen) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        fd => AsyncResult::Ok((fd, ep.clone())),
    }
}

pub fn accept_async<S, E, F, O>(soc: &S, mut ep: E, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking + 'static,
          E: AsRawSockAddr + Clone + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<(RawFd, E)>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    if soc.as_io_actor().ready_in(io, false) {
        match accept_with_nonblock(soc, &mut ep) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_in(io, true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_in(io, true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new(soc as *const S);
    soc.as_io_actor().set_in(io, obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        let soc = unsafe { &**ptr_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = accept(soc, ep);
                callback(obj, res);
            },
        }
    }));
}

pub fn accept_syncd<S, E>(soc: &S, ep: E, io: &IoService) -> io::Result<(RawFd, E)>
    where S: AsRawFd + AsIoActor + NonBlocking,
          E: AsRawSockAddr,
{
    if let Some(callback) = soc.as_io_actor().unset_in(io) {
        callback(io, HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    accept(soc, ep)
}

fn read_with_nonblock<S>(soc: &S, buf: &mut [u8]) -> AsyncResult<usize>
    where S: AsRawFd + NonBlocking,
{
    if let Err(err) = soc.native_set_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::read(soc.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len()) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::UnexpectedEof, "")),
        size => AsyncResult::Ok(size as usize),
    }
}
pub fn read_async<S, F, O>(soc: &S, buf: &mut [u8], callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    if soc.as_io_actor().ready_in(io, false) {
        match read_with_nonblock(soc, buf) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_in(io, true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_in(io, true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_mut_ptr(), buf.len()));
    soc.as_io_actor().set_in(io, obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts_mut(ptr_.1, ptr_.2) };
                    read(soc, buf)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn read_syncd<S>(soc: &S, buf: &mut [u8], io: &IoService) -> io::Result<usize>
    where S: AsRawFd + AsIoActor + NonBlocking,
{
    if let Some(callback) = soc.as_io_actor().unset_in(io) {
        callback(io, HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    read(soc, buf)
}

fn recv_with_nonblock<S>(soc: &S, buf: &mut [u8], flags: i32) -> AsyncResult<usize>
    where S: AsRawFd + NonBlocking,
{
    if let Err(err) = soc.native_set_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::recv(soc.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len(), flags) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::UnexpectedEof, "")),
        size => AsyncResult::Ok(size as usize),
    }
}

pub fn recv_async<S, F, O>(soc: &S, buf: &mut [u8], flags: i32, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    if soc.as_io_actor().ready_in(io, false) {
        match recv_with_nonblock(soc, buf, flags) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_in(io, true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_in(io, true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_mut_ptr(), buf.len()));
    soc.as_io_actor().set_in(io, obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts_mut(ptr_.1, ptr_.2) };
                    recv(soc, buf, flags)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn recv_syncd<S>(soc: &S, buf: &mut [u8], flags: i32, io: &IoService) -> io::Result<usize>
    where S: AsRawFd + AsIoActor + NonBlocking,
{
    if let Some(callback) = soc.as_io_actor().unset_in(io) {
        callback(io, HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    recv(soc, buf, flags)
}

fn recvfrom_with_nonblock<S, E>(soc: &S, buf: &mut [u8], flags: i32, ep: &mut E) -> AsyncResult<(usize, E)>
    where S: AsRawFd + NonBlocking,
          E: AsRawSockAddr + Clone {
    if let Err(err) = soc.native_set_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    let mut socklen = ep.raw_socklen();
    match unsafe { libc::recvfrom(soc.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len(), flags, ep.as_mut_raw_sockaddr(), &mut socklen) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::UnexpectedEof, "")),
        size => AsyncResult::Ok((size as usize, ep.clone())),
    }
}

pub fn recvfrom_async<S, E, F, O>(soc: &S, buf: &mut [u8], flags: i32, mut ep: E, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking + 'static,
          E: AsRawSockAddr + Send + Clone + 'static,
          F: FnOnce(Strand<O>, io::Result<(usize, E)>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    if soc.as_io_actor().ready_in(io, false) {
        match recvfrom_with_nonblock(soc, buf, flags, &mut ep) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_in(io, true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_in(io, true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_mut_ptr(), buf.len()));
    soc.as_io_actor().set_in(io, obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts_mut(ptr_.1, ptr_.2) };
                    recvfrom(soc, buf, flags, ep)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn recvfrom_syncd<S, E>(soc: &S, buf: &mut [u8], flags: i32, ep: E, io: &IoService) -> io::Result<(usize, E)>
    where S: AsRawFd + AsIoActor + NonBlocking,
          E: AsRawSockAddr,
{
    if let Some(callback) = soc.as_io_actor().unset_in(io) {
        callback(io, HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    recvfrom(soc, buf, flags, ep)
}

fn write_with_nonblock<S>(soc: &S, buf: &[u8]) -> AsyncResult<usize>
    where S: AsRawFd + NonBlocking,
{
    if let Err(err) = soc.native_set_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::write(soc.as_raw_fd(), buf.as_ptr() as *const libc::c_void, buf.len()) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::WriteZero, "")),
        size => AsyncResult::Ok(size as usize),
    }
}

pub fn write_async<S, F, O>(soc: &S, buf: &[u8], callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    if soc.as_io_actor().ready_out(io, false) {
        match write_with_nonblock(soc, buf) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_out(io, true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_out(io, true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_ptr(), buf.len()));
    soc.as_io_actor().set_out(io, obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts(ptr_.1, ptr_.2) };
                    write(soc, buf)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn write_syncd<S>(soc: &S, buf: &[u8], io: &IoService) -> io::Result<usize>
    where S: AsRawFd + AsIoActor + NonBlocking,
{
    if let Some(callback) = soc.as_io_actor().unset_out(io) {
        callback(io, HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    write(soc, buf)
}

fn send_with_nonblock<S>(soc: &S, buf: &[u8], flags: i32) -> AsyncResult<usize>
    where S: AsRawFd + NonBlocking,
{
    if let Err(err) = soc.native_set_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::send(soc.as_raw_fd(), buf.as_ptr() as *const libc::c_void, buf.len(), flags) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::WriteZero, "")),
        size => AsyncResult::Ok(size as usize),
    }
}

pub fn send_async<S, F, O>(soc: &S, buf: &[u8], flags: i32, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    if soc.as_io_actor().ready_out(io, false) {
        match send_with_nonblock(soc, buf, flags) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_out(io, true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_out(io, true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_ptr(), buf.len()));
    soc.as_io_actor().set_out(io, obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts(ptr_.1, ptr_.2) };
                    send(soc, buf, flags)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn send_syncd<S>(soc: &S, buf: &[u8], flags: i32, io: &IoService) -> io::Result<usize>
    where S: AsRawFd + AsIoActor + NonBlocking,
{
    if let Some(callback) = soc.as_io_actor().unset_out(io) {
        callback(io, HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    send(soc, buf, flags)
}

fn sendto_with_nonblock<S, E>(soc: &S, buf: &[u8], flags: i32, ep: &E) -> AsyncResult<usize>
    where S: AsRawFd + NonBlocking,
          E: AsRawSockAddr,
{
    if let Err(err) = soc.native_set_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::sendto(soc.as_raw_fd(), buf.as_ptr() as *const libc::c_void, buf.len(), flags, ep.as_raw_sockaddr(), ep.raw_socklen()) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::WriteZero, "")),
        size => AsyncResult::Ok(size as usize),
    }
}

pub fn sendto_async<S, E, F, O>(soc: &S, buf: &[u8], flags: i32, ep: &E, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking + 'static,
          E: AsRawSockAddr + Clone + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    if soc.as_io_actor().ready_out(io, false) {
        match sendto_with_nonblock(soc, buf, flags, ep) {
            AsyncResult::Err(err) => {
                soc.as_io_actor().ready_out(io, true);
                io.post_strand(move |obj| callback(obj, Err(err)), obj);
                return;
            }
            AsyncResult::Ok(res) => {
                soc.as_io_actor().ready_out(io, true);
                io.post_strand(move |obj| callback(obj, Ok(res)), obj);
                return;
            }
            AsyncResult::WouldBlock => {}
        }
    }

    let obj_ = obj.obj.clone();
    let ptr_ = UnsafeThreadableCell::new((soc as *const S, buf.as_ptr(), buf.len()));
    let ep_ = ep.clone();
    soc.as_io_actor().set_out(io, obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Errored |
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let soc = unsafe { &*(ptr_.0) };
                    let buf = unsafe { slice::from_raw_parts(ptr_.1, ptr_.2) };
                    sendto(soc, buf, flags, &ep_)
                };
                callback(obj, res);
            },
        }
    }));
}

pub fn sendto_syncd<S, E>(soc: &S, buf: &[u8], flags: i32, ep: &E, io: &IoService) -> io::Result<usize>
    where S: AsRawFd + AsIoActor + NonBlocking,
          E: AsRawSockAddr,
{
    if let Some(callback) = soc.as_io_actor().unset_out(io) {
        callback(io, HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    sendto(soc, buf, flags, ep)
}

pub fn cancel_io<S, O>(soc: &S, obj: &Strand<O>)
    where S: AsIoActor,
{
    let io = obj.io_service();
    if let Some(callback) = soc.as_io_actor().unset_in(obj.io_service()) {
        io.0.task.post(obj.id(), Box::new(move |io| callback(io, HandlerResult::Canceled)));
    }
    if let Some(callback) = soc.as_io_actor().unset_out(obj.io_service()) {
        io.0.task.post(obj.id(), Box::new(move |io| callback(io, HandlerResult::Canceled)));
    }
}

pub fn async_timer<T, F, O>(timer: &T, expiry: Expiry, callback: F, obj: &Strand<O>)
    where T: AsTimerActor + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<()>) + Send + 'static,
          O: 'static,
{
    let obj_ = obj.obj.clone();
    timer.as_timer_actor().set_timer(obj.io_service(), expiry, obj.id(), Box::new(move |io: *const IoService, res| {
        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
        match res {
            HandlerResult::Ready
                => callback(obj, Ok(())),
            HandlerResult::Canceled
                => callback(obj, Err(operation_canceled())),
            HandlerResult::Errored
                => unreachable!(),
        }
    }));
}

pub fn cancel_timer<T, O>(timer: &T, obj: &Strand<O>)
    where T: AsTimerActor,
{
    let io = obj.io_service();
    if let Some(callback) = timer.as_timer_actor().unset_timer(obj.io_service()) {
        io.0.task.post(obj.id(), Box::new(move |io| callback(io, HandlerResult::Canceled)));
    }
}

fn read_until_async_loop<S, C, F, T>(soc: &S, sbuf: &mut StreamBuf, mut cond: C, callback: F, obj: &Strand<T>, mut cur: usize)
    where S: AsRawFd + AsIoActor + NonBlocking + Send + 'static,
          C: MatchCondition + Clone + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static,
{
    let io = obj.io_service();
    match cond.is_match(&sbuf.as_slice()[cur..]) {
        Ok(len) => {
            let obj_ = obj.obj.clone();
            io.0.task.post(obj.id(), Box::new(move |io: *const IoService| {
                let obj = Strand { io: unsafe { &*io }, obj: obj_ };
                callback(obj, Ok(cur + len));
            }));
        }
        Err(len) => {
            cur += len;
            let ptr = sbuf as *mut StreamBuf;
            match sbuf.prepare_max(4096) {
                Ok(buf) => {
                    let mut ptr_ = UnsafeThreadableCell::new((soc as *const S, ptr));
                    read_async(soc, buf, move |obj, res| {
                        match res {
                            Ok(len) => {
                                let soc = unsafe { &*ptr_.0 };
                                let sbuf = unsafe { &mut *ptr_.1 };
                                read_until_async_loop(soc, sbuf, cond, callback, &obj, cur + len);
                            }
                            Err(err) => {
                                let obj_ = obj.obj.clone();
                                obj.io_service().0.task.post(obj.id(), Box::new(|io: *const IoService| {
                                    let obj = Strand { io: unsafe { &*io }, obj: obj_ };
                                    callback(obj, Err(err));
                                }));
                            }
                        }
                    }, obj)
                },
                Err(err) => {
                    let obj_ = obj.obj.clone();
                    io.0.task.post(obj.id(), Box::new(|io: *const IoService| {
                        let obj = Strand { io: unsafe { &*io }, obj: obj_ };
                        callback(obj, Err(err));
                    }));
                },
            }
        }
    }
}

pub fn read_until_async<S, C, F, T>(soc: &S, sbuf: &mut StreamBuf, cond: C, callback: F, obj: &Strand<T>)
    where S: AsRawFd + AsIoActor + NonBlocking + Send + 'static,
          C: MatchCondition + Clone + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static,
{
    read_until_async_loop(soc, sbuf, cond, callback, obj, 0)
}

fn write_until_async_loop<S, F, T>(soc: &S, sbuf: &mut StreamBuf, mut rest: usize, callback: F, obj: &Strand<T>, mut cur: usize)
    where S: AsRawFd + AsIoActor + NonBlocking + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static,
{
    let ptr = sbuf as *mut StreamBuf;
    let buf = &sbuf.as_slice()[..cur];
    let mut ptr_ = UnsafeThreadableCell::new((soc as *const S, ptr));
    write_async(soc, buf, move |obj, res| {
        match res {
            Ok(len) => {
                let soc = unsafe { &*ptr_.0 };
                let sbuf = unsafe { &mut *ptr_.1 };
                sbuf.consume(len);
                rest -= len;
                cur += len;
                if cur == 0 {
                    callback(obj, Ok(cur));
                } else {
                    write_until_async_loop(soc, sbuf, rest, callback, &obj, cur);
                }
            },
            Err(err) => {
                let io = obj.io_service();
                let obj_ = obj.obj.clone();
                io.0.task.post(obj.id(), Box::new(|io: *const IoService| {
                    let obj = Strand { io: unsafe { &*io }, obj: obj_ };
                    callback(obj, Err(err))
                }));
            }
        }
    }, obj);
}

pub fn write_until_async<S, C, F, T>(soc: &S, sbuf: &mut StreamBuf, mut cond: C, callback: F, obj: &Strand<T>)
    where S: AsRawFd + AsIoActor + NonBlocking + Send + 'static,
          C: MatchCondition + Clone + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static,
{
    let len = match cond.is_match(sbuf.as_slice()) {
        Ok(len) => len,
        Err(len) => len,
    };
    write_until_async_loop(soc, sbuf, len, callback, obj, 0)
}
