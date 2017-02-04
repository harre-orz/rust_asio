use super::{Dispatcher, FdContext, IntrFd, AsyncFd};
use unsafe_cell::UnsafeBoxedCell;
use ffi::{RawFd, AsRawFd, read, close};
use error::{ErrCode, READY, ECANCELED, sock_error};
use core::{AsIoContext, Scheduler, Interrupter, Operation, Ops, ThreadIoContext};

use std::io;
use std::mem;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use std::collections::HashSet;
use libc::{epoll_event, epoll_create1, epoll_ctl, epoll_wait,
           EPOLLIN, EPOLLOUT, EPOLLERR, EPOLLHUP, EPOLLET,
           EPOLL_CLOEXEC, EPOLL_CTL_ADD, EPOLL_CTL_DEL};

pub type Dispatch = fn(&epoll_event, &mut ThreadIoContext);

pub struct EpollReactor {
    epfd: RawFd,
    mutex: Mutex<HashSet<UnsafeBoxedCell<FdContext>>>,
    outstanding_work: Arc<AtomicUsize>,
}

impl Drop for EpollReactor {
    fn drop(&mut self) {
        close(self.epfd);
    }
}

impl EpollReactor {
    pub fn new(outstanding_work: Arc<AtomicUsize>) -> io::Result<Self> {
        Ok(EpollReactor {
            epfd: libc_try!(epoll_create1(EPOLL_CLOEXEC)),
            mutex: Default::default(),
            outstanding_work: outstanding_work,
        })
    }

    fn epoll_ctl(&self, fd: &FdContext, op: i32, events: i32) {
        let mut ev = epoll_event {
            events: events as u32,
            u64: fd as *const _ as u64,
        };
        libc_ign!(epoll_ctl(self.epfd, op, fd.as_raw_fd(), &mut ev));
    }

    pub fn register_intr_fd(&self, fd: &IntrFd) {
        self.epoll_ctl(fd, EPOLL_CTL_ADD, EPOLLIN);
    }

    pub fn deregister_intr_fd(&self, fd: &IntrFd) {
        self.epoll_ctl(fd, EPOLL_CTL_DEL, EPOLLIN);
    }

    pub fn register_async_fd(&self, fd: &AsyncFd) {
        self.epoll_ctl(fd, EPOLL_CTL_ADD, EPOLLIN | EPOLLOUT | EPOLLET);
        let mut ep = self.mutex.lock().unwrap();
        ep.insert(fd.0.clone());
    }

    pub fn deregister_async_fd(&self, fd: &AsyncFd) {
        self.epoll_ctl(fd, EPOLL_CTL_DEL, 0);
        let mut ep = self.mutex.lock().unwrap();
        ep.remove(&fd.0);
    }

    pub fn cancel_all_fds(&self, this: &mut ThreadIoContext) {
        let ep = self.mutex.lock().unwrap();
        for fd in ep.iter() {
            fd.clone().clear_all(this, ECANCELED);
        }
    }

    pub fn run(&self, schd: &Scheduler, block: bool, this: &mut ThreadIoContext) {
        let timeout = if block {
            let timeout = schd.wait_duration(Duration::new(10,0));
            timeout.as_secs() as i32 * 1000 + (timeout.subsec_nanos() / 1000000) as i32
        } else {
            0
        };

        let mut events: [epoll_event; 128] = unsafe { mem::uninitialized() };
        let n = unsafe {
            epoll_wait(self.epfd, events.as_mut_ptr(), events.len() as i32, timeout)
        };

        schd.get_ready_timers(this);

        if n > 0 {
            let this_len = this.len();
            {
                let mut _ep = self.mutex.lock().unwrap();
                for ev in &events[..(n as usize)] {
                    (from_event(ev).dispatch)(ev, this);
                }
            }
            self.outstanding_work.fetch_sub(this.len() - this_len, Ordering::SeqCst);
        }
    }

    fn add_op(&self, this: &mut ThreadIoContext, ops: &mut Ops, op: Operation, ec: ErrCode) {
        if ec == READY {
            ops.canceled = false;

            let _ep = self.mutex.lock().unwrap();
            if ops.queue.is_empty() && !ops.blocked {
                ops.blocked = true;
                this.push(op, READY);
            } else {
                self.outstanding_work.fetch_add(1, Ordering::SeqCst);
                ops.queue.push_back(op);
            }
        } else {
            if !ops.canceled {
                let _ep = self.mutex.lock().unwrap();
                self.outstanding_work.fetch_add(1, Ordering::SeqCst);
                ops.queue.push_front(op);
                ops.blocked = false;
            } else {
                ops.canceled = false;

                let _ep = self.mutex.lock().unwrap();
                ops.queue.push_front(op);
                self.outstanding_work.fetch_sub(ops.queue.len(), Ordering::SeqCst);
                for op in ops.queue.drain(..) {
                    this.push(op, ECANCELED)
                }
                ops.blocked = false;
            }
        }
    }

    fn next_op(&self, this: &mut ThreadIoContext, ops: &mut Ops)
    {
        if !ops.canceled {
            let _ep = self.mutex.lock().unwrap();
            if let Some(op) = ops.queue.pop_front() {
                this.push(op, READY);
                self.outstanding_work.fetch_sub(1, Ordering::SeqCst);
            } else {
                ops.blocked = false;
            }
        } else {
            ops.canceled = false;

            let _kq = self.mutex.lock().unwrap();
            self.outstanding_work.fetch_sub(ops.queue.len(), Ordering::SeqCst);
            for op in ops.queue.drain(..) {
                this.push(op, ECANCELED);
            }
            ops.blocked = false;
        }
    }

    fn cancel_op(&self, this: &mut ThreadIoContext, ops: &mut Ops) {
        if ops.canceled {
            return;
        }
        ops.canceled = true;

        let _ep = self.mutex.lock().unwrap();
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

impl Dispatcher for Scheduler {
    fn dispatcher() -> Dispatch { dispatch_schd }
}

fn dispatch_async(ev: &epoll_event, this: &mut ThreadIoContext) {
    let fd = from_event(ev);
    if ev.events & (EPOLLERR | EPOLLHUP) as u32 != 0 {
        let ec = sock_error(fd);
        fd.clear_all(this, ec);
        return;
    }

    if ev.events & EPOLLIN as u32 != 0 {
        fd.ready_input(this);
    }
    if ev.events & EPOLLOUT as u32 != 0 {
        fd.ready_output(this);
    }
}

fn dispatch_intr(ev: &epoll_event, _: &mut ThreadIoContext) {
    if ev.events & EPOLLIN as u32 != 0 {
        let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
        libc_ign!(read(from_event(ev), &mut buf));
    }
}

fn dispatch_schd(ev: &epoll_event, _: &mut ThreadIoContext) {
    if ev.events & EPOLLIN as u32 != 0 {
        let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
        libc_ign!(read(from_event(ev), &mut buf));
    }
}

fn from_event(ev: &epoll_event) -> &mut FdContext {
    unsafe { &mut *(ev.u64 as *mut FdContext) }
}


impl AsyncFd {
    pub fn add_input_op(&self, this: &mut ThreadIoContext, op: Operation, ec: ErrCode) {
        self.as_ctx().0.reactor.add_op(this, &mut self.0.clone().input, op, ec)
    }

    pub fn add_output_op(&self, this: &mut ThreadIoContext, op: Operation, ec: ErrCode) {
        self.as_ctx().0.reactor.add_op(this, &mut self.0.clone().output, op, ec)
    }

    pub fn next_input_op(&self, this: &mut ThreadIoContext) {
        self.as_ctx().0.reactor.next_op(this, &mut self.0.clone().input)
    }

    pub fn next_output_op(&self, this: &mut ThreadIoContext) {
        self.as_ctx().0.reactor.next_op(this, &mut self.0.clone().output)
    }

    pub fn cancel_input_op(&self, this: &mut ThreadIoContext) {
        self.as_ctx().0.reactor.cancel_op(this, &mut self.0.clone().input)
    }

    pub fn cancel_output_op(&self, this: &mut ThreadIoContext) {
        self.as_ctx().0.reactor.cancel_op(this, &mut self.0.clone().output)
    }
}
