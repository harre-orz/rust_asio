use ffi::*;
use core::{get_ready_timers, wait_duration, Expiry, IoContext, AsIoContext, ThreadIoContext,
           Perform, TimerQueue, Intr};

use std::mem;
use std::ptr;
use std::time::Duration;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::{HashSet, VecDeque};
use libc::{self, EV_ADD, EV_DELETE, EV_ERROR, EV_ENABLE, EV_DISPATCH, EV_CLEAR, EVFILT_READ,
           EVFILT_WRITE, EVFILT_SIGNAL, SIG_SETMASK, sigset_t, sigemptyset, sigaddset, sigprocmask};

fn dispatch_socket(kev: &libc::kevent, this: &mut ThreadIoContext) {
    let soc: &mut Kevent = unsafe { &mut *(kev.udata as *mut _ as *mut Kevent) };
    if (kev.flags & EV_ERROR) != 0 {
        let err = sock_error(soc);
        soc.input.blocked = false;
        soc.input.canceled = false;
        for op in soc.input.queue.drain(..) {
            this.push(op, err.clone())
        }
        soc.output.blocked = false;
        soc.output.canceled = false;
        for op in soc.output.queue.drain(..) {
            this.push(op, err.clone())
        }
    } else if kev.filter == EVFILT_READ {
        if let Some(op) = soc.input.queue.pop_front() {
            soc.input.blocked = true;
            this.push(op, SystemError::default())
        } else {
            soc.input.blocked = false;
        }
    } else if kev.filter == EVFILT_WRITE {
        if let Some(op) = soc.output.queue.pop_front() {
            soc.output.blocked = true;
            this.push(op, SystemError::default())
        } else {
            soc.output.blocked = false;
        }
    } else if kev.filter == EVFILT_SIGNAL {
        if let Some(op) = soc.input.queue.pop_front() {
            let sig: Signal = unsafe { mem::transmute(kev.ident as i32) };
            this.push(op, SystemError::from_signal(sig));
        }
    }
}

fn dispatch_intr(kev: &libc::kevent, _: &mut ThreadIoContext) {
    if kev.filter == EVFILT_READ {
        let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
        unsafe { libc::read(kev.ident as RawFd, buf.as_mut_ptr() as *mut _, buf.len()) };
    }
}

#[derive(Default)]
pub struct Ops {
    queue: VecDeque<Box<Perform>>,
    blocked: bool,
    canceled: bool,
}

pub struct Kevent {
    fd: RawFd,
    input: Ops,
    output: Ops,
    dispatch: fn(&libc::kevent, &mut ThreadIoContext),
}

impl Kevent {
    pub fn socket(fd: RawFd) -> Self {
        Kevent {
            fd: fd,
            input: Default::default(),
            output: Default::default(),
            dispatch: dispatch_socket,
        }
    }

    pub fn intr(fd: RawFd) -> Self {
        Kevent {
            fd: fd,
            input: Default::default(),
            output: Default::default(),
            dispatch: dispatch_intr,
        }
    }

    pub fn signal() -> Self {
        Kevent {
            fd: -1,
            input: Default::default(),
            output: Default::default(),
            dispatch: dispatch_socket,
        }
    }

    pub fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        let kq = unsafe { &*(this.as_ctx().as_reactor() as *const KqueueReactor) };
        let _kq = kq.mutex.lock().unwrap();

        let ops = &mut unsafe { &mut *(self as *const _ as *mut Self) }.input;
        if err == SystemError::default() {
            if ops.queue.is_empty() && !ops.blocked {
                ops.blocked = true;
                this.push(op, SystemError::default());
            } else {
                ops.queue.push_back(op);
            }
        } else if ops.canceled {
            ops.queue.push_front(op);
            for op in ops.queue.drain(..) {
                this.push(op, OPERATION_CANCELED);
            }
            this.as_ctx().as_reactor().kevent(
                KeventPtr(self),
                EV_ENABLE,
                EVFILT_READ,
            );
        } else {
            ops.blocked = false;
            ops.queue.push_front(op);
            this.as_ctx().as_reactor().kevent(
                KeventPtr(self),
                EV_ENABLE,
                EVFILT_READ,
            );
        }
    }

    pub fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        let kq = unsafe { &*(this.as_ctx().as_reactor() as *const KqueueReactor) };
        let _kq = kq.mutex.lock().unwrap();

        let ops = &mut unsafe { &mut *(self as *const _ as *mut Self) }.output;
        if err == SystemError::default() {
            if ops.queue.is_empty() && !ops.blocked {
                ops.blocked = true;
                this.push(op, SystemError::default());
            } else {
                ops.queue.push_back(op);
            }
        } else if ops.canceled {
            ops.queue.push_front(op);
            for op in ops.queue.drain(..) {
                this.push(op, OPERATION_CANCELED);
            }
            this.as_ctx().as_reactor().kevent(
                KeventPtr(self),
                EV_ENABLE,
                EVFILT_WRITE,
            );
        } else {
            ops.blocked = false;
            ops.queue.push_front(op);
            this.as_ctx().as_reactor().kevent(
                KeventPtr(self),
                EV_ENABLE,
                EVFILT_WRITE,
            );
        }
    }

    pub fn cancel_ops(&self, ctx: &IoContext) {
        let kq = unsafe { &*(ctx.as_reactor() as *const KqueueReactor) };
        let _kq = kq.mutex.lock().unwrap();

        for ops in &mut [
            &mut unsafe { &mut *(self as *const _ as *mut Self) }.input,
            &mut unsafe { &mut *(self as *const _ as *mut Self) }.output,
        ]
        {
            if !ops.canceled {
                ops.canceled = true;
                if !ops.blocked {
                    for op in ops.queue.drain(..) {
                        ctx.do_post((op, OPERATION_CANCELED))
                    }
                }
            }
        }
    }

    pub fn next_read_op(&self, this: &mut ThreadIoContext) {
        let kq = unsafe { &*(this.as_ctx().as_reactor() as *const KqueueReactor) };
        let _kq = kq.mutex.lock().unwrap();

        let ops = &mut unsafe { &mut *(self as *const _ as *mut Self) }.input;
        if ops.canceled {
            ops.canceled = false;
            for op in ops.queue.drain(..) {
                this.push(op, OPERATION_CANCELED);
            }
        } else {
            if let Some(op) = ops.queue.pop_front() {
                this.push(op, SystemError::default());
            } else {
                ops.blocked = false;
                this.as_ctx().as_reactor().kevent(
                    KeventPtr(self),
                    EV_ENABLE,
                    EVFILT_READ,
                );
            }
        }
    }

    pub fn next_write_op(&self, this: &mut ThreadIoContext) {
        let kq = unsafe { &*(this.as_ctx().as_reactor() as *const KqueueReactor) };
        let _kq = kq.mutex.lock().unwrap();

        let ops = &mut unsafe { &mut *(self as *const _ as *mut Self) }.output;
        if ops.canceled {
            ops.canceled = false;
            for op in ops.queue.drain(..) {
                this.push(op, OPERATION_CANCELED);
            }
        } else {
            if let Some(op) = ops.queue.pop_front() {
                this.push(op, SystemError::default());
            } else {
                ops.blocked = false;
                this.as_ctx().as_reactor().kevent(
                    KeventPtr(self),
                    EV_ENABLE,
                    EVFILT_WRITE,
                );
            }
        }
    }
}

unsafe impl Send for Kevent {}

impl AsRawFd for Kevent {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

pub struct KeventPtr(*const Kevent);

unsafe impl Send for KeventPtr {}

unsafe impl Sync for KeventPtr {}

impl PartialEq for KeventPtr {
    fn eq(&self, other: &KeventPtr) -> bool {
        self.0 == other.0
    }
}

impl Eq for KeventPtr {}

impl Hash for KeventPtr {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_usize(self.0 as usize)
    }
}

impl AsRawFd for KeventPtr {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { &*self.0 }.as_raw_fd()
    }
}

fn make_kev(soc: &KeventPtr, flags: u16, filter: i16) -> libc::kevent {
    libc::kevent {
        ident: unsafe { &*soc.0 }.as_raw_fd() as usize,
        filter: filter,
        flags: flags,
        fflags: 0,
        data: 0,
        udata: soc.0 as *const _ as *mut _,
    }
}

fn make_sig(soc: &KeventPtr, flags: u16, sig: i32) -> libc::kevent {
    libc::kevent {
        ident: sig as usize,
        filter: EVFILT_SIGNAL,
        flags: flags,
        fflags: 0,
        data: 0,
        udata: soc.0 as *const _ as *mut _,
    }
}

mod pipe;
pub use self::pipe::PipeIntr;

mod socket;
pub use self::socket::KqueueSocket;

mod signal;
pub use self::signal::KqueueSignal;

mod reactor;
pub use self::reactor::KqueueReactor;
