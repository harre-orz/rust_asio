use super::{Dispatcher, FdContext, IntrFd, AsyncFd};
use unsafe_cell::UnsafeBoxedCell;
use ffi::{RawFd, AsRawFd, read, close};
use error::{ErrCode, READY, ECANCELED, sock_error};
use core::{AsIoContext, ThreadIoContext, Scheduler, Interrupter, Operation, Ops};

use std::io;
use std::mem;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use std::collections::HashSet;
use libc::{EV_ADD, EV_DELETE, EV_ERROR, EV_CLEAR, EV_ENABLE, EV_DISPATCH,
           EVFILT_READ, EVFILT_WRITE, kqueue, kevent, timespec};

pub type Dispatch = fn(&kevent, &mut ThreadIoContext);

pub struct KqueueReactor {
    kq: RawFd,
    mutex: Mutex<HashSet<UnsafeBoxedCell<FdContext>>>,
    outstanding_work: Arc<AtomicUsize>,
}

impl Drop for KqueueReactor {
    fn drop(&mut self) {
        close(self.kq);
    }
}

impl KqueueReactor {
    pub fn new(outstanding_work: Arc<AtomicUsize>) -> io::Result<Self> {
        Ok(KqueueReactor {
            kq: libc_try!(kqueue()),
            mutex: Default::default(),
            outstanding_work: outstanding_work,
        })
    }

    fn kevent(&self, fd: &FdContext, flags: u16, filter: i16)
    {
        let kev = make_kev(fd, flags, filter);
        libc_ign!(kevent(self.kq, &kev, 1, ptr::null_mut(), 0, ptr::null()));
    }

    fn kevent_both(&self, fd: &FdContext, flags: u16)
    {
        let kev = [
            make_kev(fd, flags, EVFILT_READ),
            make_kev(fd, flags, EVFILT_WRITE),
        ];
        libc_ign!(kevent(self.kq, kev.as_ptr(), 2, ptr::null_mut(), 0, ptr::null()));
    }

    pub fn register_intr_fd(&self, fd: &IntrFd) {
        self.kevent(fd, EV_ADD | EV_CLEAR | EV_ENABLE, EVFILT_READ);
    }

    pub fn deregister_intr_fd(&self, fd: &IntrFd) {
        self.kevent(fd, EV_DELETE, EVFILT_READ);
    }

    pub fn register_async_fd(&self, fd: &AsyncFd) {
        self.kevent_both(fd, EV_ADD | EV_CLEAR | EV_ENABLE | EV_DISPATCH);
        let mut kq = self.mutex.lock().unwrap();
        kq.insert(fd.0.clone());
    }

    pub fn deregister_async_fd(&self, fd: &AsyncFd) {
        self.kevent_both(fd, EV_DELETE);
        let mut kq = self.mutex.lock().unwrap();
        kq.remove(&fd.0);
    }

    pub fn cancel_all_fds(&self, this: &mut ThreadIoContext) {
        let kq = self.mutex.lock().unwrap();
        for fd in kq.iter() {
            fd.clone().clear_all(this, ECANCELED);
        }
    }

    pub fn run(&self, schd: &Scheduler, block: bool, this: &mut ThreadIoContext) {
        let tv = if block {
            let timeout = schd.wait_duration(Duration::new(10,0));
            timespec {
                tv_sec: timeout.as_secs() as _,
                tv_nsec: timeout.subsec_nanos() as _,
            }
        } else {
            timespec { tv_sec: 0, tv_nsec: 0 }
        };

        let mut kev: [kevent; 128] = unsafe { mem::uninitialized() };
        let n = unsafe {
            kevent(self.kq, ptr::null(), 0, kev.as_mut_ptr(), kev.len() as _, &tv)
        };

        schd.get_ready_timers(this);

        if n > 0 {
            let len = this.len();
            {
                let _kq = self.mutex.lock().unwrap();
                for ev in &kev[..(n as usize)] {
                    (from_kev(ev).dispatch)(ev, this);
                }
            }
            self.outstanding_work.fetch_sub(this.len() - len, Ordering::SeqCst);
        }
    }

    pub fn add_op(&self, this: &mut ThreadIoContext, ops: &mut Ops, op: Operation,
                  ec: ErrCode, fd: &AsyncFd, filter: i16)
    {
        if ec == READY {
            ops.canceled = false;

            let _kq = self.mutex.lock().unwrap();
            if ops.queue.is_empty() && !ops.blocked {
                ops.blocked = true;
                this.push(op, READY);
            } else {
                self.outstanding_work.fetch_add(1, Ordering::SeqCst);
                ops.queue.push_back(op);
            }
        } else {
            if !ops.canceled {
                let _kq = self.mutex.lock().unwrap();
                self.outstanding_work.fetch_add(1, Ordering::SeqCst);
                ops.queue.push_front(op);
                ops.blocked = false;
                self.kevent(fd, EV_ENABLE, filter);
            } else {
                ops.canceled = false;

                let _kq = self.mutex.lock().unwrap();
                ops.queue.push_front(op);
                self.outstanding_work.fetch_sub(ops.queue.len(), Ordering::SeqCst);
                for op in ops.queue.drain(..) {
                    this.push(op, ECANCELED)
                }
                ops.blocked = false;
                self.kevent(fd, EV_ENABLE, filter);
            }
        }
    }

    pub fn next_op(&self, this: &mut ThreadIoContext, ops: &mut Ops,
                   fd: &AsyncFd, filter: i16)
    {
        if !ops.canceled {
            let _kq = self.mutex.lock().unwrap();
            if let Some(op) = ops.queue.pop_front() {
                this.push(op, READY);
                self.outstanding_work.fetch_sub(1, Ordering::SeqCst);
            } else {
                ops.blocked = false;
                self.kevent(fd, EV_ENABLE, filter);
            }
        } else {
            ops.canceled = false;

            let _kq = self.mutex.lock().unwrap();
            self.outstanding_work.fetch_sub(ops.queue.len(), Ordering::SeqCst);
            for op in ops.queue.drain(..) {
                this.push(op, ECANCELED);
            }
            ops.blocked = false;
            self.kevent(fd, EV_ENABLE, filter);
        }
    }

    pub fn cancel_op(&self, this: &mut ThreadIoContext, ops: &mut Ops) {
        if ops.canceled {
            return;
        }
        ops.canceled = true;

        let _kq = self.mutex.lock().unwrap();
        if !ops.blocked {
            self.outstanding_work.fetch_sub(ops.queue.len(), Ordering::SeqCst);
            for op in ops.queue.drain(..) {
                this.push(op, ECANCELED);
            }
        }
    }
}

impl<T: AsIoContext> Dispatcher for T {
    fn dispatcher() -> Dispatch { dispatch_async }
}

impl Dispatcher for Interrupter {
    fn dispatcher() -> Dispatch { dispatch_intr }
}

fn dispatch_async(kev: &kevent, this: &mut ThreadIoContext) {
    let fd = from_kev(kev);
    if (kev.flags & EV_ERROR) != 0 {
        let ec = sock_error(fd);
        fd.clear_all(this, ec);
    } else if kev.filter == EVFILT_READ {
        fd.ready_input(this);
    } else if kev.filter == EVFILT_WRITE {
        fd.ready_output(this);
    }
}

fn dispatch_intr(kev: &kevent, _: &mut ThreadIoContext) {
    if kev.filter == EVFILT_READ {
        let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
        libc_ign!(read(from_kev(kev), &mut buf));
    }
}

fn make_kev(fd: &FdContext, flags: u16, filter: i16) -> kevent {
    kevent {
        ident: fd.as_raw_fd() as _,
        filter: filter,
        flags: flags,
        fflags: 0,
        data: 0,
        udata: fd as *const _ as *mut _,
    }
}

fn from_kev(ev: &kevent) -> &mut FdContext {
    unsafe { &mut *(ev.udata as *const _ as *mut _) }
}

impl AsyncFd {
    pub fn add_input_op(&self, this: &mut ThreadIoContext, op: Operation, ec: ErrCode) {
        self.as_ctx().0.reactor.add_op(this, &mut self.0.clone().input,
                                       op, ec, self, EVFILT_READ);
    }

    pub fn add_output_op(&self, this: &mut ThreadIoContext, op: Operation, ec: ErrCode) {
        self.as_ctx().0.reactor.add_op(this, &mut self.0.clone().output,
                                       op, ec, self, EVFILT_WRITE);
    }

    pub fn next_input_op(&self, this: &mut ThreadIoContext) {
        self.as_ctx().0.reactor.next_op(this, &mut self.0.clone().input,
                                        self, EVFILT_READ)
    }

    pub fn next_output_op(&self, this: &mut ThreadIoContext) {
        self.as_ctx().0.reactor.next_op(this, &mut self.0.clone().output,
                                        self, EVFILT_WRITE)
    }

    pub fn cancel_input_op(&self, this: &mut ThreadIoContext) {
        self.as_ctx().0.reactor.cancel_op(this, &mut self.0.clone().input)
    }

    pub fn cancel_output_op(&self, this: &mut ThreadIoContext) {
        self.as_ctx().0.reactor.cancel_op(this, &mut self.0.clone().output)
    }
}
