use std::io;
use std::cmp;
use {IoObject, IoService, Strand};
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

struct SendWrap<T> {
    obj: Box<T>,
}

unsafe impl<T> Send for SendWrap<T> {}

fn connect_with_nonblock<S, E>(soc: &S, ep: &E) -> AsyncResult<()>
    where S: AsRawFd + NonBlocking,
          E: AsRawSockAddr,
{
    if let Err(err) = soc.native_set_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::connect(soc.as_raw_fd(), ep.as_raw_sockaddr(), ep.raw_socklen()) } {
        -1 => {
            let err = errno();
            if err == libc::EINPROGRESS {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        _ => AsyncResult::Ok(()),
    }
}

pub fn connect_async<S, P, E, A, F, O>(as_ref: A, ep: &E, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking,
          P: Protocol + Send + 'static,
          E: Endpoint<P> + AsRawSockAddr,
          A: Fn(&O) -> &S + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<()>) + Send + 'static,
          O: 'static
{
    let io = obj.io_service();
    let soc = as_ref(&*obj);
    if let Some(callback) = soc.as_io_actor().unset_out(io) {
        io.0.task.post(obj.id(), Box::new(move || callback(HandlerResult::Canceled)));
    }
    match connect_with_nonblock(soc, ep) {
        AsyncResult::Err(err) => {
            io.post_strand(move |obj| callback(obj, Err(err)), obj);
        },
        AsyncResult::Ok(_) => {
            io.post_strand(move |obj| callback(obj, Ok(())), obj);
        },
        AsyncResult::WouldBlock => {
            let arc = obj.0.clone();
            let pro = ep.protocol();
            soc.as_io_actor().set_out(io, obj.id(), Box::new(move |res| {
                let obj = Strand(arc);
                match res {
                    HandlerResult::Canceled => {
                        if let Ok(fd) = socket(pro) {
                            as_ref(&*obj).as_io_actor().reopen(fd);
                        }
                        callback(obj, Err(operation_canceled()));
                    },
                    HandlerResult::Ready => callback(obj, Ok(())),
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
        callback(HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    connect(soc, ep)
}

fn accept_with_nonblock<S, E>(soc: &S, mut ep: E) -> AsyncResult<(RawFd, E)>
    where S: AsRawFd + NonBlocking,
          E: AsRawSockAddr,
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
        fd => AsyncResult::Ok((fd, ep)),
    }
}

pub fn accept_async<S, E, A, F, O>(as_ref: A, ep: E, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking,
          E: AsRawSockAddr + Clone + Send + 'static,
          A: Fn(&O) -> &S + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<(RawFd, E)>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    let soc = as_ref(&*obj);
    let ep2 = ep.clone();
    if soc.as_io_actor().ready_in(io, false) {
        match accept_with_nonblock(soc, ep) {
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

    let arc = obj.0.clone();
    soc.as_io_actor().set_in(io, obj.id(), Box::new(move |res| {
        let obj = Strand(arc);
        match res {
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = accept(as_ref(&*obj), ep2);
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
        callback(HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    accept(soc, ep)
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

pub fn recv_async<S, A, F, O>(as_ref: A, flags: i32, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking,
          A: Fn(&mut O) -> (&S, &mut [u8]) + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    let (soc, buf) = as_ref(obj.get_mut());
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

    let arc = obj.0.clone();
    soc.as_io_actor().set_in(io, obj.id(), Box::new(move |res| {
        let obj = Strand(arc);
        match res {
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let (soc, buf) = as_ref(obj.get_mut());
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
        callback(HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    recv(soc, buf, flags)
}


fn recvfrom_with_nonblock<S, E>(soc: &S, buf: &mut [u8], flags: i32, mut ep: E) -> AsyncResult<(usize, E)>
    where S: AsRawFd + NonBlocking,
          E: AsRawSockAddr {
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
        size => AsyncResult::Ok((size as usize, ep)),
    }
}

pub fn recvfrom_async<S, A, E, F, O>(as_ref: A, flags: i32, ep: E, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking,
          A: Fn(&mut O) -> (&S, &mut [u8]) + Send + 'static,
          E: AsRawSockAddr + Send + Clone + 'static,
          F: FnOnce(Strand<O>, io::Result<(usize, E)>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    let (soc, buf) = as_ref(obj.get_mut());
    let ep2 = ep.clone();
    if soc.as_io_actor().ready_in(io, false) {
        match recvfrom_with_nonblock(soc, buf, flags, ep) {
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

    let arc = obj.0.clone();
    soc.as_io_actor().set_in(io, obj.id(), Box::new(move |res| {
        let obj = Strand(arc);
        match res {
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let (soc, buf) = as_ref(obj.get_mut());
                    recvfrom(soc, buf, flags, ep2)
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
        callback(HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    recvfrom(soc, buf, flags, ep)
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

pub fn send_async<S, A, F, O>(as_ref: A, flags: i32, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking,
          A: Fn(&O) -> (&S, &[u8]) + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    let (soc, buf) = as_ref(obj.get_mut());
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

    let arc = obj.0.clone();
    soc.as_io_actor().set_out(io, obj.id(), Box::new(move |res| {
        let obj = Strand(arc);
        match res {
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let (soc, buf) = as_ref(obj.get_mut());
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
        callback(HandlerResult::Canceled);
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

pub fn sendto_async<S, A, E, F, O>(as_ref: A, flags: i32, ep: &E, callback: F, obj: &Strand<O>)
    where S: AsRawFd + AsIoActor + NonBlocking,
          A: Fn(&O) -> (&S, &[u8]) + Send + 'static,
          E: AsRawSockAddr + Clone + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let io = obj.io_service();
    let (soc, buf) = as_ref(obj.get_mut());
    let ep2 = ep.clone();
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

    let arc = obj.0.clone();
    soc.as_io_actor().set_out(io, obj.id(), Box::new(move |res| {
        let obj = Strand(arc);
        match res {
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => {
                let res = {
                    let (soc, buf) = as_ref(obj.get_mut());
                    sendto(soc, buf, flags, &ep2)
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
        callback(HandlerResult::Canceled);
    }
    try!(setnonblock(soc, soc.get_non_blocking()));
    sendto(soc, buf, flags, ep)
}

pub fn cancel_io<S, A, O>(as_ref: A, obj: &Strand<O>)
    where S: AsIoActor + 'static,
          A: Fn(&O) -> &S + 'static,
          O: 'static,
{
    let io = obj.io_service();
    let soc = as_ref(&*obj);
    if let Some(callback) = soc.as_io_actor().unset_in(obj.io_service()) {
        io.0.task.post(obj.id(), Box::new(move || callback(HandlerResult::Canceled)));
    }
    if let Some(callback) = soc.as_io_actor().unset_out(obj.io_service()) {
        io.0.task.post(obj.id(), Box::new(move || callback(HandlerResult::Canceled)));
    }
}

pub fn async_resolve<I, T, A, Q, F, O>(_: A, query: Q, callback: F, obj: &Strand<O>)
    where T: Send + 'static,
          A: Fn(&O) -> &T + Send + 'static,
          Q: FnOnce() -> io::Result<I> + 'static,
          F: FnOnce(Strand<O>, io::Result<I>) + Send + 'static,
          O: 'static,
{
    let wrap = SendWrap { obj: Box::new(query) };
    let arc = obj.0.clone();
    obj.io_service().post_strand(move |_| {
        callback(Strand(arc), (*wrap.obj)());
    }, obj);
}

pub fn async_timer<T, A, F, O>(as_ref: A, expiry: Expiry, callback: F, obj: &Strand<O>)
    where T: AsTimerActor + Send + 'static,
          A: Fn(&O) -> &T + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<()>) + Send + 'static,
          O: 'static,
{
    let arc = obj.0.clone();
    as_ref(&*obj).as_timer_actor().set_timer(obj.io_service(), expiry, obj.id(), Box::new(move |res| {
        let obj = Strand(arc);
        match res {
            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
            HandlerResult::Ready => callback(obj, Ok(())),
        }
    }));
}

pub fn cancel_timer<T, A, O>(as_ref: A, obj: &Strand<O>)
    where T: AsTimerActor,
          A: Fn(&O) -> &T + 'static,
          O: 'static,
{
    let io = obj.io_service();
    if let Some(callback) = as_ref(&*obj).as_timer_actor().unset_timer(obj.io_service()) {
        io.0.task.post(obj.id(), Box::new(move || callback(HandlerResult::Canceled)));
    }
}

pub fn read_until_async<S, A, C, F, T>(as_ref: A, mut cond: C, callback: F, obj: &Strand<T>, cur: usize)
    where S: AsRawFd + AsIoActor + NonBlocking,
          A: Fn(&mut T) -> (&S, &mut StreamBuf) + Send + 'static,
          C: MatchCondition + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static,
{
    let io = obj.io_service();
    let (soc, sbuf) = as_ref(obj.get_mut());
    let arc = obj.0.clone();
    match cond.is_match(&sbuf.as_slice()[cur..]) {
        Ok(len) =>
            io.0.task.post(obj.id(), Box::new(move || callback(Strand(arc), Ok(cur + len)))),
        Err(len) => {
            let cur = cmp::min(cur+len, sbuf.len());
            match sbuf.prepare(4096) {
                Err(err) =>
                    io.0.task.post(obj.id(), Box::new(move || callback(Strand(arc), Err(err)))),
                Ok(buf) => {
                    if soc.as_io_actor().ready_in(io, false) {
                        match recv_with_nonblock(soc, buf, 0) {
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
                    soc.as_io_actor().set_in(io, obj.id(), Box::new(move |res| {
                        let obj = Strand(arc);
                        match res {
                            HandlerResult::Canceled => callback(obj, Err(operation_canceled())),
                            HandlerResult::Ready => {
                                match {
                                    let (soc, sbuf) = as_ref(obj.get_mut());
                                    recv(soc, sbuf.prepare(4096).unwrap(), 0)
                                } {
                                    Err(err) => callback(obj, Err(err)),
                                    Ok(len) => {
                                        let (_, sbuf) = as_ref(obj.get_mut());
                                        sbuf.commit(len);
                                        read_until_async(as_ref, cond, callback, &obj, cur);
                                    }
                                }
                            },
                        }
                    }));
                }
            }
        }
    }
}
