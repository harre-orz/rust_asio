use super::{Kevent, kevent};
use ffi::{AsRawFd, RawFd, pipe, close, SystemError, OPERATION_CANCELED};
use core::{get_ready_timers, wait_duration, IoContext, AsIoContext, ThreadIoContext, TimerQueue, Perform, Expiry};


use std::mem;
use std::ptr;
use std::time::Duration;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::ops::{Deref, DerefMut};
use std::collections::{HashSet};
use libc::{self, EV_ADD, EV_DELETE, EV_ENABLE, EV_DISPATCH, EV_CLEAR, EVFILT_READ,
           EVFILT_WRITE, sigset_t, sigemptyset};

struct PipeIntr {
    rfd: Kevent,
    wfd: RawFd,
}

impl PipeIntr {
    pub fn new() -> Result<Self, SystemError> {
        let (rfd, wfd) = pipe()?;
        Ok(PipeIntr {
            rfd: Kevent::intr(rfd),
            wfd: wfd,
        })
    }

    pub fn startup(&self, reactor: &KqueueReactor) {
        reactor.register_intr(&self.rfd);
    }

    pub fn cleanup(&self, reactor: &KqueueReactor) {
        reactor.deregister_intr(&self.rfd)
    }

    pub fn interrupt(&self) {
        unsafe {
            let buf: [u8; 1] = mem::uninitialized();
            libc::write(self.wfd, buf.as_ptr() as *const libc::c_void, buf.len());
        }
    }
}

impl Drop for PipeIntr {
    fn drop(&mut self) {
        close(self.rfd.as_raw_fd());
        close(self.wfd);
    }
}

struct KeventPtr(*const Kevent);

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

impl Deref for KeventPtr {
    type Target = Kevent;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl DerefMut for KeventPtr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.0 as *mut _)}
    }
}


pub struct KqueueReactor {
    kq: RawFd,
    mutex: Mutex<HashSet<KeventPtr>>,
    intr: PipeIntr,
    pub tq: Mutex<TimerQueue>,
    pub sigmask: Mutex<sigset_t>,
}

impl KqueueReactor {
    pub fn new() -> Result<Self, SystemError> {
        match unsafe { libc::kqueue() } {
            -1 => Err(SystemError::last_error()),
            kq => {
                let kq = KqueueReactor {
                    tq: Mutex::new(TimerQueue::new()),
                    kq: kq,
                    sigmask: unsafe {
                        let mut sigmask = mem::uninitialized();
                        sigemptyset(&mut sigmask);
                        Mutex::new(sigmask)
                    },
                    mutex: Default::default(),
                    intr: PipeIntr::new()?,
                };
                kq.intr.startup(&kq);
                Ok(kq)
            }
        }
    }

    pub fn kevent(&self, kev: &[libc::kevent]) {
        unsafe { libc::kevent(self.kq, kev.as_ptr(), kev.len() as i32, ptr::null_mut(), 0, ptr::null()) };
    }

    pub fn poll(&self, block: bool, this: &mut ThreadIoContext) {
        let tv = if block {
            let timeout = wait_duration(&self.tq, Duration::new(10, 0));
            libc::timespec {
                tv_sec: timeout.as_secs() as _,
                tv_nsec: timeout.subsec_nanos() as _,
            }
        } else {
            libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            }
        };

        let mut kev: [libc::kevent; 128] = unsafe { mem::uninitialized() };
        let n = unsafe {
            libc::kevent(
                self.kq,
                ptr::null(),
                0,
                kev.as_mut_ptr(),
                kev.len() as _,
                &tv,
            )
        };

        self.get_ready_timers(this);

        if n > 0 {
            let _kq = self.mutex.lock().unwrap();
            for ev in &kev[..(n as usize)] {
                let soc = unsafe { &*(ev.udata as *const Kevent) };
                (soc.dispatch)(ev, this)
            }
        }
    }

    fn get_ready_timers(&self, this: &mut ThreadIoContext) {
        let mut tq = self.tq.lock().unwrap();
        get_ready_timers(&mut tq, this, Expiry::now());
    }

    pub fn register_socket(&self, fd: &Kevent) {
        self.kevent(&[kevent(fd, EVFILT_READ, EV_ADD | EV_CLEAR | EV_ENABLE | EV_DISPATCH),
                      kevent(fd, EVFILT_WRITE, EV_ADD | EV_CLEAR | EV_ENABLE | EV_DISPATCH)]);
        let mut kq = self.mutex.lock().unwrap();
        kq.insert(KeventPtr(fd));
    }

    pub fn deregister_socket(&self, fd: &Kevent) {
        self.kevent(&[kevent(fd, EVFILT_READ, EV_DELETE), kevent(fd, EVFILT_WRITE, EV_DELETE)]);
        let mut kq = self.mutex.lock().unwrap();
        kq.remove(&KeventPtr(fd));
    }

    pub fn register_signal(&self, fd: &Kevent) {
        let mut kq = self.mutex.lock().unwrap();
        kq.insert(KeventPtr(fd));
    }

    pub fn deregister_signal(&self, fd: &Kevent) {
        let mut kq = self.mutex.lock().unwrap();
        kq.remove(&KeventPtr(fd));
    }

    pub fn register_intr(&self, fd: &Kevent) {
        self.kevent(&[kevent(fd, EVFILT_READ, EV_ADD | EV_CLEAR)]);
    }

    pub fn deregister_intr(&self, fd: &Kevent) {
        self.kevent(&[kevent(fd, EVFILT_READ, EV_DELETE | EV_CLEAR)]);
    }

    pub fn interrupt(&self) {
        self.intr.interrupt()
    }

    pub fn reset_timeout(&self, expiry: Expiry) {
        self.intr.interrupt()
    }

    pub fn add_read_op(&self, kev: &Kevent, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        let _kq = self.mutex.lock().unwrap();
        let ops = &mut KeventPtr(kev).input;
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
            this.as_ctx().as_reactor().kevent(&[kevent(kev, EVFILT_READ, EV_ENABLE)])
        } else {
            ops.blocked = false;
            ops.queue.push_front(op);
            this.as_ctx().as_reactor().kevent(&[kevent(kev, EVFILT_READ, EV_ENABLE)])
        }
    }

    pub fn add_write_op(&self, kev: &Kevent, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        let _kq = self.mutex.lock().unwrap();
        let ops = &mut KeventPtr(kev).output;
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
            this.as_ctx().as_reactor().kevent(&[kevent(kev, EVFILT_WRITE, EV_ENABLE)]);
        } else {
            ops.blocked = false;
            ops.queue.push_front(op);
            this.as_ctx().as_reactor().kevent(&[kevent(kev, EVFILT_WRITE, EV_ENABLE)])
        }
    }

    pub fn next_read_op(&self, kev: &Kevent, this: &mut ThreadIoContext) {
        let _kq = self.mutex.lock().unwrap();
        let ops = &mut KeventPtr(kev).input;
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
                this.as_ctx().as_reactor().kevent(&[kevent(kev, EVFILT_READ, EV_ENABLE)]);
            }
        }
    }

    pub fn next_write_op(&self, kev: &Kevent, this: &mut ThreadIoContext) {
        let _kq = self.mutex.lock().unwrap();
        let ops = &mut KeventPtr(kev).output;
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
                this.as_ctx().as_reactor().kevent(&[kevent(kev, EVFILT_READ, EV_ENABLE)]);
            }
        }
    }

    pub fn cancel_ops(&self, kev: &Kevent, ctx: &IoContext, err: SystemError) {
        let _kq = self.mutex.lock().unwrap();
        for ops in &mut [&mut KeventPtr(kev).input, &mut KeventPtr(kev).output] {
            if !ops.canceled {
                ops.canceled = true;
                if !ops.blocked {
                    for op in ops.queue.drain(..) {
                        ctx.do_post((op, err))
                    }
                }
            }
        }
    }
}

impl Drop for KqueueReactor {
    fn drop(&mut self) {
        self.intr.cleanup(self);
        close(self.kq);
    }
}
