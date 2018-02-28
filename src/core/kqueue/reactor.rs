use super::*;
use ffi::{AsRawFd, RawFd, close, SystemError, Signal};
use core::{get_ready_timers, wait_duration, AsIoContext, IoContext, ThreadIoContext, TimerQueue, Perform, Expiry};


use std::mem;
use std::ptr;
use std::time::Duration;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::{HashSet, VecDeque};
use libc::{self, EV_ADD, EV_DELETE, EV_ERROR, EV_ENABLE, EV_DISPATCH, EV_CLEAR, EVFILT_READ,
           EVFILT_WRITE, EVFILT_SIGNAL, SIG_SETMASK, sigset_t, sigemptyset, sigaddset, sigprocmask};



pub struct KqueueReactor {
    pub kq: RawFd,
    pub mutex: Mutex<HashSet<KeventPtr>>,
    pub intr: PipeIntr,
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

    pub fn kevent(&self, fd: KeventPtr, flags: u16, filter: i16) {
        let kev = make_kev(&fd, flags, filter);
        unsafe { libc::kevent(self.kq, &kev, 1, ptr::null_mut(), 0, ptr::null()) };
    }

    pub fn kevent_both(&self, fd: KeventPtr, flags: u16) {
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
        self.kevent_both(KeventPtr(fd), EV_ADD | EV_CLEAR | EV_ENABLE | EV_DISPATCH);
        let mut kq = self.mutex.lock().unwrap();
        kq.insert(KeventPtr(fd));
    }

    pub fn deregister_socket(&self, fd: &Kevent) {
        self.kevent_both(KeventPtr(fd), EV_DELETE);
        let mut kq = self.mutex.lock().unwrap();
        kq.remove(&KeventPtr(fd));
    }

    pub fn register_intr(&self, fd: &Kevent) {
        self.kevent(KeventPtr(fd), EV_ADD | EV_CLEAR | EV_ENABLE, EVFILT_READ);
    }

    pub fn deregister_intr(&self, fd: &Kevent) {
        self.kevent(KeventPtr(fd), EV_DELETE, EVFILT_READ);
    }

    pub fn register_signal(&self, fd: &Kevent) {
        let mut kq = self.mutex.lock().unwrap();
        kq.insert(KeventPtr(fd));
    }

    pub fn deregister_signal(&self, fd: &Kevent) {
        let mut kq = self.mutex.lock().unwrap();
        kq.remove(&KeventPtr(fd));
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
