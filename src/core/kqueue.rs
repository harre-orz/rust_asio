use ffi::*;
use core::{get_ready_timers, wait_duration, Expiry, IoContext, AsIoContext, ThreadIoContext, Perform, TimerQueue, Intr};

use std::mem;
use std::ptr;
use std::time::Duration;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::collections::{HashSet, VecDeque};
use libc::{
    self,
    EV_ADD,
    EV_DELETE,
    EV_ERROR,
    EV_ENABLE,
    EV_DISPATCH,
    EV_CLEAR,
    EVFILT_READ,
    EVFILT_WRITE,
    EVFILT_SIGNAL,
};

#[derive(Default)]
pub struct Ops {
    queue: VecDeque<Box<Perform>>,
    blocked: bool,
    canceled: bool,
}

pub struct KqueueFd {
    fd: RawFd,
    input: Ops,
    output: Ops,
    dispatch: fn(&libc::kevent, &mut ThreadIoContext),
}

impl KqueueFd {
    pub fn socket(fd: RawFd) -> Self {
        KqueueFd {
            fd: fd,
            input: Default::default(),
            output: Default::default(),
            dispatch: dispatch_socket,
        }
    }

    pub fn intr(fd: RawFd) -> Self {
        KqueueFd {
            fd: fd,
            input: Default::default(),
            output: Default::default(),
            dispatch: dispatch_intr,
        }
    }

    pub fn signal() -> Self {
        KqueueFd {
            fd: -1,
            input: Default::default(),
            output: Default::default(),
            dispatch: dispatch_signal,
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
            this.as_ctx().as_reactor().kevent(KqueueFdPtr(self), EV_ENABLE, EVFILT_READ);
        } else {
            ops.blocked = false;
            ops.queue.push_front(op);
            this.as_ctx().as_reactor().kevent(KqueueFdPtr(self), EV_ENABLE, EVFILT_READ);
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
            this.as_ctx().as_reactor().kevent(KqueueFdPtr(self), EV_ENABLE, EVFILT_WRITE);
        } else {
            ops.blocked = false;
            ops.queue.push_front(op);
            this.as_ctx().as_reactor().kevent(KqueueFdPtr(self), EV_ENABLE, EVFILT_WRITE);
        }
    }

    pub fn cancel_ops(&self, ctx: &IoContext) {
        let kq = unsafe { &*(ctx.as_reactor() as *const KqueueReactor) };
        let _kq = kq.mutex.lock().unwrap();

        for ops in &mut [
            &mut unsafe { &mut *(self as *const _ as *mut Self) }.input,
            &mut unsafe { &mut *(self as *const _ as *mut Self) }.output
        ] {
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
                this.as_ctx().as_reactor().kevent(KqueueFdPtr(self), EV_ENABLE, EVFILT_READ);
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
                this.as_ctx().as_reactor().kevent(KqueueFdPtr(self), EV_ENABLE, EVFILT_WRITE);
            }
        }
    }
}

impl Drop for KqueueFd {
    fn drop(&mut self) {
        close(self.fd)
    }
}

unsafe impl Send for KqueueFd {}

impl AsRawFd for KqueueFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}


pub struct KqueueFdPtr(*const KqueueFd);

unsafe impl Send for KqueueFdPtr {}

unsafe impl Sync for KqueueFdPtr {}

impl PartialEq for KqueueFdPtr {
    fn eq(&self, other: &KqueueFdPtr) -> bool {
        self.0 == other.0
    }
}

impl Eq for KqueueFdPtr {}

impl Hash for KqueueFdPtr {
    fn hash<H>(&self, state: &mut H)
        where H: Hasher
    {
        state.write_usize(self.0 as usize)
    }
}

impl AsRawFd for KqueueFdPtr {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { &*self.0 }.as_raw_fd()
    }
}

fn make_kev(soc: &KqueueFdPtr, flags: u16, filter: i16) -> libc::kevent {
    libc::kevent {
        ident: unsafe { &*soc.0 }.as_raw_fd() as usize,
        filter: filter,
        flags: flags,
        fflags: 0,
        data: 0,
        udata: soc.0 as *const _ as *mut _,
    }
}

fn make_sig(soc: &KqueueFdPtr, flags: u16, sig: i32) -> libc::kevent {
    libc::kevent {
        ident: sig as usize,
        filter: EVFILT_SIGNAL,
        flags: flags,
        fflags: 0,
        data: 0,
        udata: soc.0 as *const _ as *mut _,
    }
}

fn dispatch_socket(kev: &libc::kevent, this: &mut ThreadIoContext) {
    let soc: &mut KqueueFd = unsafe { &mut *(kev.udata as *mut _ as *mut KqueueFd) };
    if (kev.flags & EV_ERROR) != 0 {
        let err = sock_error(soc);
        println!("sock_error {}", err);
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
    }
}


fn dispatch_intr(kev: &libc::kevent, _: &mut ThreadIoContext) {
    if kev.filter == EVFILT_READ {
        let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
        unsafe { libc::read(kev.ident as RawFd, buf.as_mut_ptr() as *mut _, buf.len()) };
    }
}

fn dispatch_signal(kev: &libc::kevent, this: &mut ThreadIoContext) {
    if kev.filter == EVFILT_SIGNAL {
        // if let Some(op) = soc.input.queue.pop_front() {
        //     soc.input.blocked = true;
        //     this.push(op, SystemError::default())
    //}
    }
}

pub struct KqueueReactor {
    pub tq: Mutex<TimerQueue>,
    kq: RawFd,
    mutex: Mutex<HashSet<KqueueFdPtr>>,
    intr: Intr,
}

impl KqueueReactor {
    pub fn new() -> Result<Self, SystemError> {
        match unsafe { libc::kqueue() } {
            -1 => Err(SystemError::last_error()),
            kq => {
                let kq = KqueueReactor {
                    tq: Mutex::new(TimerQueue::new()),
                kq: kq,
                    mutex: Default::default(),
                    intr: Intr::new()?,
                };
                kq.intr.startup(&kq);
                Ok(kq)
            }
        }
    }

    fn kevent(&self, fd: KqueueFdPtr, flags: u16, filter: i16) {
        let kev = make_kev(&fd, flags, filter);
        unsafe { libc::kevent(self.kq, &kev, 1, ptr::null_mut(), 0, ptr::null()) };
    }

    fn kevent_both(&self, fd: KqueueFdPtr, flags: u16) {
        let kev = [
            make_kev(&fd, flags, EVFILT_READ),
            make_kev(&fd, flags, EVFILT_WRITE),
        ];
        unsafe { libc::kevent(self.kq, kev.as_ptr(), 2, ptr::null_mut(), 0, ptr::null()) };
    }

    pub fn poll(&self, block: bool, this: &mut ThreadIoContext) {
        let tv = if block {
            let timeout = wait_duration(&self.tq, Duration::new(10, 0));
            libc::timespec {
                tv_sec: timeout.as_secs() as _,
                tv_nsec: timeout.subsec_nanos()  as _,
            }
        } else {
            libc::timespec { tv_sec: 0, tv_nsec: 0 }
        };

        let mut kev: [libc::kevent; 128] = unsafe { mem::uninitialized() };
        let n = unsafe {
            libc::kevent(self.kq, ptr::null(), 0, kev.as_mut_ptr(), kev.len() as _, &tv)
        };

        self.get_ready_timers(this);

        if n > 0 {
            let _kq = self.mutex.lock().unwrap();
            for ev in &kev[..(n as usize)] {
                let soc = unsafe { &*(ev.udata as *const KqueueFd) };
                (soc.dispatch)(ev, this)
            }
        }
    }

    fn get_ready_timers(&self, this: &mut ThreadIoContext) {
        let mut tq = self.tq.lock().unwrap();
        get_ready_timers(&mut tq, this, Expiry::now());
    }

    pub fn register_socket(&self, fd: &KqueueFd) {
        self.kevent_both(KqueueFdPtr(fd), EV_ADD | EV_CLEAR | EV_ENABLE | EV_DISPATCH);
        let mut kq = self.mutex.lock().unwrap();
        kq.insert(KqueueFdPtr(fd));
    }

    pub fn deregister_socket(&self, fd: &KqueueFd) {
        self.kevent_both(KqueueFdPtr(fd), EV_DELETE);
        let mut kq = self.mutex.lock().unwrap();
        kq.remove(&KqueueFdPtr(fd));
    }

    pub fn register_intr(&self, fd: &KqueueFd) {
        self.kevent(KqueueFdPtr(fd), EV_ADD | EV_CLEAR | EV_ENABLE, EVFILT_READ);
    }

    pub fn deregister_intr(&self, fd: &KqueueFd) {
        self.kevent(KqueueFdPtr(fd), EV_DELETE, EVFILT_READ);
    }

    pub fn interrupt(&self) {
        self.intr.interrupt()
    }

    pub fn reset_timeout(&self, expiry: Expiry) {
        self.intr.interrupt()
    }
}

impl Drop for KqueueReactor {
    fn drop(&mut self) {
        self.intr.cleanup(self);
        close(self.kq);
    }
}
