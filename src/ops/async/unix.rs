use std::io;
use {IoObject, IoService, Strand};
use backbone::{Expiry, EpollIoActor, TimerActor};
use libc;
use ops::*;

pub trait AsIoActor {
    fn as_io_actor(&self) -> &EpollIoActor;
}

pub trait AsTimerActor {
    fn as_timer_actor(&self) -> &TimerActor;
}

enum ConnectStatus {
    Complete,
    Inprogress,
}

fn connect_with_nonblock<Fd: AsRawFd, E: AsRawSockAddr>(fd: &Fd, ep: &E) -> io::Result<ConnectStatus> {
    try!(setnonblock(fd, true));
    match unsafe { libc::connect(fd.as_raw_fd(), ep.as_raw_sockaddr(), ep.raw_socklen()) } {
        0 => Ok(ConnectStatus::Complete),
        _ => match errno() {
            libc::EINPROGRESS => Ok(ConnectStatus::Inprogress),
            errno => Err(io::Error::from_raw_os_error(errno)),
        }
    }
}

pub fn async_connect<S, E, A, F, O>(as_ref: A, ep: &E, callback: F, obj: &Strand<O>)
    where S: IoObject + AsRawFd + AsIoActor,
          E: AsRawSockAddr,
          A: Fn(&O) -> &S + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<()>) + Send + 'static,
          O: 'static,
{
    let soc = as_ref(&*obj);
    let io = soc.io_service();
    if let Some(callback) = soc.as_io_actor().unset_out() {
        io.post_strand(move || {
            callback(Err(operation_canceled()));
        }, obj)
    }

    let is_block = match getnonblock(soc) {
        Ok(res) => res,
        Err(err) => {
            let obj = obj.clone();
            io.post(move || {
                callback(obj, Err(err));
            });
            return;
        }
    };

    match connect_with_nonblock(soc, ep) {
        Err(err) => {
            if is_block {
                let _ = setnonblock(soc, false);
            }
            let _obj = obj.clone();
            io.post_strand(move || {
                callback(_obj, Err(err));
            }, obj)
        }
        Ok(st) => match st {
            ConnectStatus::Complete => {
                if is_block {
                    let _ = setnonblock(soc, false);
                }
                let _obj = obj.clone();
                io.post_strand(move || {
                    callback(_obj, Ok(()));
                }, &obj)
            },
            ConnectStatus::Inprogress => {
                let _obj = obj.clone();
                if let Some(callback) = soc.as_io_actor().set_out(Box::new(move |res| {
                    {
                        let soc = as_ref(&*_obj);
                        if is_block {
                            let _ = setnonblock(soc, false);
                        }
                    }
                    match res {
                        Ok(_) => callback(_obj, Ok(())),
                        Err(err) => callback(_obj, Err(err)),
                    }
                }), obj.id()) {
                    io.post_strand(move || {
                        callback(Err(operation_canceled()));
                    }, obj)
                }
            }
        }
    }
}

pub fn async_recv<S, A, F, O>(as_ref: A, flags: i32, callback: F, obj: &Strand<O>)
    where S: IoObject + AsRawFd + AsIoActor,
          A: Fn(&mut O) -> (&S, &mut [u8]) + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let (soc, _) = as_ref(obj.get_mut());
    let io = soc.io_service();
    let mut _obj = obj.clone();
    if let Some(callback) = soc.as_io_actor().set_in(Box::new(move |res| {
        match res {
            Ok(_) => {
                let res = {
                    let (soc, buf) = as_ref(_obj.get_mut());
                    recv(soc, buf, flags)
                };
                callback(_obj, res);
            },
            Err(err) => callback(_obj, Err(err)),
        }
    }), obj.id()) {
        io.post_strand(move || {
            callback(Err(operation_canceled()));
        }, obj)
    }
}

pub fn async_recvfrom<S, A, E, F, O>(as_ref: A, flags: i32, ep: E, callback: F, obj: &Strand<O>)
    where S: IoObject + AsRawFd + AsIoActor,
          A: Fn(&mut O) -> (&S, &mut [u8]) + Send + 'static,
          E: AsRawSockAddr + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<(usize, E)>) + Send + 'static,
          O: 'static,
{
    let (soc, _) = as_ref(obj.get_mut());
    let io = soc.io_service();
    let mut _obj = obj.clone();
    if let Some(callback) = soc.as_io_actor().set_in(Box::new(move |res| {
        match res {
            Ok(_) => {
                let res = {
                    let (soc, buf) = as_ref(_obj.get_mut());
                    recvfrom(soc, buf, flags, ep)
                };
                callback(_obj, res);
            },
            Err(err) => callback(_obj, Err(err)),
        }
    }), obj.id()) {
        io.post_strand(move || {
            callback(Err(operation_canceled()));
        }, obj)
    }
}

pub fn async_send<S, A, F, O>(as_ref: A, flags: i32, callback: F, obj: &Strand<O>)
    where S: IoObject + AsRawFd + AsIoActor,
          A: Fn(&O) -> (&S, &[u8]) + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let (soc, _) = as_ref(&*obj);
    let io = soc.io_service();
    let mut _obj = obj.clone();
    if let Some(callback) = soc.as_io_actor().set_out(Box::new(move |res| {
        match res {
            Ok(_) => {
                let res = {
                    let (soc, buf) = as_ref(&*_obj);
                    send(soc, buf, flags)
                };
                callback(_obj, res);
            },
            Err(err) => callback(_obj, Err(err)),
        }
    }), obj.id()) {
        io.post_strand(move || {
            callback(Err(operation_canceled()));
        }, obj)
    }
}

pub fn async_sendto<S, A, E, F, O>(as_ref: A, flags: i32, ep: &E, callback: F, obj: &Strand<O>)
    where S: IoObject + AsRawFd + AsIoActor,
          A: Fn(&O) -> (&S, &[u8]) + Send + 'static,
          E: AsRawSockAddr + Clone + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<usize>) + Send + 'static,
          O: 'static,
{
    let ep = ep.clone();
    let (soc, _) = as_ref(&*obj);
    let io = soc.io_service();
    let mut _obj = obj.clone();
    if let Some(callback) = soc.as_io_actor().set_out(Box::new(move |res| {
        match res {
            Ok(_) => {
                let res = {
                    let (soc, buf) = as_ref(&*_obj);
                    sendto(soc, buf, flags, &ep)
                };
                callback(_obj, res);
            },
            Err(err) => callback(_obj, Err(err)),
        }
    }), obj.id()) {
        io.post_strand(move || {
            callback(Err(operation_canceled()));
        }, obj)
    }
}

pub fn async_accept<S, E, A, F, O>(as_ref: A, ep: E, callback: F, obj: &Strand<O>)
    where S: IoObject + AsRawFd + AsIoActor,
          E: AsRawSockAddr + Send + 'static,
          A: Fn(&O) -> &S + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<(IoService, RawFd, E)>) + Send + 'static,
          O: 'static,
{
    let soc = as_ref(&*obj);
    let io = soc.io_service();
    let mut _obj = obj.clone();
    if let Some(callback) = soc.as_io_actor().set_in(Box::new(move |res| {
        match res {
            Ok(_) => {
                let res = {
                    let soc = as_ref(&*_obj);
                    accept(soc, ep)
                };
                callback(_obj, res);
            },
            Err(err) => callback(_obj, Err(err)),
        }
    }), obj.id()) {
        io.post_strand(move || {
            callback(Err(operation_canceled()));
        }, obj)
    }
}

pub fn cancel_io<S, A, O>(as_ref: A, obj: &Strand<O>)
    where S: IoObject + AsIoActor,
          A: Fn(&O) -> &S,
{
    let soc = as_ref(&*obj);
    let io = soc.io_service();
    if let Some(callback) = soc.as_io_actor().unset_in() {
        io.post_strand(move || {
            callback(Err(operation_canceled()))
        }, obj)
    }
    if let Some(callback) = soc.as_io_actor().unset_out() {
        io.post_strand(move || {
            callback(Err(operation_canceled()))
        }, obj)
    }
}

pub fn async_timer<T, A, F, O>(as_ref: A, expiry: Expiry, callback: F, obj: &Strand<O>)
    where T: IoObject + AsTimerActor + Send + 'static,
          A: Fn(&O) -> &T + Send + 'static,
          F: FnOnce(Strand<O>, io::Result<()>) + Send + 'static,
          O: 'static {
    let timer = as_ref(&*obj);
    let io = timer.io_service();
    let _obj = obj.clone();
    if let Some(callback) = timer.as_timer_actor().set_timer(expiry, Box::new(move |res| {
        callback(_obj, res);
    }), obj.id()) {
        io.post_strand(move || {
            callback(Err(operation_canceled()))
        }, obj)
    }
}

pub fn cancel_timer<T, A, O>(as_ref: A, obj: &Strand<O>)
    where T: IoObject + AsTimerActor,
          A: Fn(&O) -> &T + 'static {
    let timer = as_ref(&*obj);
    let io = timer.io_service();
    if let Some(callback) = timer.as_timer_actor().unset_timer() {
        io.post_strand(move || {
            callback(Err(operation_canceled()))
        }, obj)
    }
}
