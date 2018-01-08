use super::{IoContext, AsIoContext, ThreadIoContext, Perform};
use ffi::*;

use std::mem;
use std::ptr;
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
    pub fn new(fd: RawFd) -> Self {
        KqueueFd {
            fd: fd,
            input: Default::default(),
            output: Default::default(),
            dispatch: soc_dispatch,
        }
    }

    pub fn add_read_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        let kq = unsafe { &*(&this.as_ctx().0.reactor as *const KqueueReactor) };
        let _kq = kq.mutex.lock().unwrap();

        if err == SystemError::default() {
            if self.input.queue.is_empty() && !self.input.blocked {
                self.input.blocked = true;
                this.push_back(op, SystemError::default());
            } else {
                self.input.queue.push_back(op);
            }
        } else if self.input.canceled {
            self.input.queue.push_front(op);
            for op in self.input.queue.drain(..) {
                this.push_back(op, OPERATION_CANCELED);
            }
            this.as_ctx().0.reactor.kevent(KqueueFdPtr(self), EV_ENABLE, EVFILT_READ);
        } else {
            self.input.blocked = false;
            self.input.queue.push_front(op);
            this.as_ctx().0.reactor.kevent(KqueueFdPtr(self), EV_ENABLE, EVFILT_READ);
        }
    }

    pub fn add_write_op(&mut self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        let kq = unsafe { &*(&this.as_ctx().0.reactor as *const KqueueReactor) };
        let _kq = kq.mutex.lock().unwrap();

        if err == SystemError::default() {
            if self.output.queue.is_empty() && !self.output.blocked {
                self.output.blocked = true;
                this.push_back(op, SystemError::default());
            } else {
                self.output.queue.push_back(op);
            }
        } else if self.output.canceled {
            self.output.queue.push_front(op);
            for op in self.input.queue.drain(..) {
                this.push_back(op, OPERATION_CANCELED);
            }
            this.as_ctx().0.reactor.kevent(KqueueFdPtr(self), EV_ENABLE, EVFILT_READ);
        } else {
            self.output.blocked = false;
            self.output.queue.push_front(op);
            this.as_ctx().0.reactor.kevent(KqueueFdPtr(self), EV_ENABLE, EVFILT_READ);
        }
    }

    pub fn cancel_read_ops(&mut self, this: &mut ThreadIoContext) {
        let kq = unsafe { &*(&this.as_ctx().0.reactor as *const KqueueReactor) };
        let _kq = kq.mutex.lock().unwrap();

        self.input.canceled = true;
        if !self.input.blocked {
            for op in self.input.queue.drain(..) {
                this.push_back(op, OPERATION_CANCELED);
            }
        }
    }

    pub fn cancel_write_ops(&mut self, this: &mut ThreadIoContext) {
        let kq = unsafe { &*(&this.as_ctx().0.reactor as *const KqueueReactor) };
        let _kq = kq.mutex.lock().unwrap();

        self.output.canceled = true;
        if !self.output.blocked {
            for op in self.output.queue.drain(..) {
                this.push_back(op, OPERATION_CANCELED);
            }
        }
    }

    pub fn next_read_op(&mut self, this: &mut ThreadIoContext) {
    }

    pub fn next_write_op(&mut self, this: &mut ThreadIoContext) {
    }
}

unsafe impl Send for KqueueFd { }

impl AsRawFd for KqueueFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}


pub struct KqueueFdPtr(*const KqueueFd);

unsafe impl Send for KqueueFdPtr { }

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


fn soc_dispatch(kev: &libc::kevent, this: &mut ThreadIoContext) {
    let soc: &mut KqueueFd = unsafe { &mut *(kev.udata as *mut _ as *mut KqueueFd) };
    if (kev.flags & EV_ERROR) != 0 {
    } else if kev.filter == EVFILT_READ {
    } else if kev.filter == EVFILT_WRITE {
    }
}


fn intr_dispatch(kev: &libc::kevent, this: &mut ThreadIoContext) {
    if kev.filter == EVFILT_READ {
        let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
        unsafe { libc::read(kev.ident as RawFd, buf.as_mut_ptr() as *mut _, buf.len()) };
    }
}


pub struct KqueueReactor {
    kq: RawFd,
    mutex: Mutex<HashSet<KqueueFdPtr>>,
}

impl KqueueReactor {
    pub fn new() -> Result<Self, SystemError> {
        match unsafe { libc::kqueue() } {
            -1 => Err(SystemError::last_error()),
            kq => Ok(KqueueReactor {
                kq: kq,
                mutex: Default::default(),
            })
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
            libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            }
        } else {
            libc::timespec { tv_sec: 0, tv_nsec: 0 }
        };

        let mut kev: [libc::kevent; 128] = unsafe { mem::uninitialized() };
        let n = unsafe {
            libc::kevent(self.kq, ptr::null(), 0, kev.as_mut_ptr(), kev.len() as _, &tv)
        };

        if n > 0 {
            let _kq = self.mutex.lock().unwrap();
            for ev in &kev[..(n as usize)] {
                let soc = unsafe { &*(ev.udata as *const KqueueFd) };
                (soc.dispatch)(ev, this)
            }
        }
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
}
