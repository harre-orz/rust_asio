use ffi::*;
use core::{IoContext, AsIoContext, ThreadIoContext, Perform};

use std::mem;
use std::collections::{VecDeque};
use libc::{self, EV_ERROR, EVFILT_READ,
           EVFILT_WRITE, EVFILT_SIGNAL};


fn kevent(soc: &Kevent, filter: i16, flags: u16) -> libc::kevent {
    libc::kevent {
        ident: soc.as_raw_fd() as usize,
        filter: filter,
        flags: flags,
        fflags: 0,
        data: 0,
        udata: soc as *const _ as *mut _,
    }
}

fn dispatch_socket(kev: &libc::kevent, this: &mut ThreadIoContext) {
    let udata = unsafe { &mut *(kev.udata as *const Kevent as *mut Kevent) };
    match kev.filter {
        _ if (kev.flags & EV_ERROR) != 0 => {
            let err = sock_error(udata);
            udata.cancel_ops(this.as_ctx(), err)
        },
        EVFILT_READ =>
            if let Some(op) = udata.input.queue.pop_front() {
                udata.input.blocked = true;
                this.push(op, SystemError::default());
            },
        EVFILT_WRITE =>
            if let Some(op) = udata.output.queue.pop_front() {
                udata.output.blocked = true;
                this.push(op, SystemError::default());
            },
        EVFILT_SIGNAL =>
            if let Some(op) = udata.input.queue.pop_front() {
                this.push(op, SystemError::from_signal(kev.ident as i32));
            },
        _ => unreachable!(),
    }
}

fn dispatch_intr(kev: &libc::kevent, _: &mut ThreadIoContext) {
    match kev.filter {
        EVFILT_READ => unsafe {
            let mut buf: [u8; 8] = mem::uninitialized();
            libc::read(kev.ident as RawFd, buf.as_mut_ptr() as *mut _, buf.len());
        },
        _ => unreachable!(),
    }
}

#[derive(Default)]
struct Ops {
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
            input: Ops {
                queue: Default::default(),
                blocked: true,  // Always blocked
                canceled: false,
            },
            output: Default::default(),
            dispatch: dispatch_socket,
        }
    }

    pub fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        this.as_ctx().clone().as_reactor().add_read_op(self, this, op, err)
    }

    pub fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        this.as_ctx().clone().as_reactor().add_write_op(self, this, op, err)
    }

    pub fn next_read_op(&self, this: &mut ThreadIoContext) {
        this.as_ctx().clone().as_reactor().next_read_op(self, this)
    }

    pub fn next_write_op(&self, this: &mut ThreadIoContext) {
        this.as_ctx().clone().as_reactor().next_write_op(self, this)
    }

    pub fn cancel_ops(&self, ctx: &IoContext, err: SystemError) {
        ctx.clone().as_reactor().cancel_ops(self, ctx, err)
    }
}

unsafe impl Send for Kevent {}

impl AsRawFd for Kevent {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

mod socket;
pub use self::socket::KqueueSocket;

mod signal;
pub use self::signal::KqueueSignal;

mod reactor;
pub use self::reactor::KqueueReactor;
