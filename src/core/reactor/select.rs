use unsafe_cell::UnsafeBoxedCell;
use ffi::{AsRawFd, FdSet, select, timeval, read, INVALID_SOCKET};
use error::{ErrCode, sock_error, READY, ECANCELED};
use core::{AsIoContext, Dispatcher, AsyncFd, IntrFd, Ops, Operation,
           FdContext, Interrupter, Scheduler, ThreadIoContext};

use std::io;
use std::mem;
use std::ptr;
use std::cmp;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

const READ: usize = 0;
const WRITE: usize = 1;
const EXCEPT: usize = 2;

pub type Dispatch = fn(&mut FdContext, &[FdSet;3], &mut ThreadIoContext);

pub struct SelectReactor {
    mutex: Mutex<Vec<UnsafeBoxedCell<FdContext>>>,
    outstanding_work: Arc<AtomicUsize>,
}

impl SelectReactor {
    pub fn new(outstanding_work: Arc<AtomicUsize>) -> io::Result<Self> {
        Ok(SelectReactor {
            mutex: Default::default(),
            outstanding_work: outstanding_work,
        })
    }

    pub fn run(&self, schd: &Scheduler, block: bool, this: &mut ThreadIoContext) {
        let mut fdset = [FdSet::new(), FdSet::new(), FdSet::new()];
        let tv = if block {
            let timeout = schd.wait_duration(Duration::new(10, 0));
            if timeout.as_secs() == 0 && timeout.subsec_nanos() < 1000000 {
                timeval { tv_sec: 0, tv_usec: 1000 }
            } else {
                timeval {
                    tv_sec: timeout.as_secs() as _,
                    tv_usec: (timeout.subsec_nanos() / 1000) as _,
                }
            }
        } else {
            timeval { tv_sec: 0, tv_usec: 1000 }
        };

        {
            // lock
            let fds = self.mutex.lock().unwrap();
            for fd in fds.iter() {
                let fd: &FdContext = fd;
                let mut except_flag = false;
                if !fd.input.blocked && !fd.input.queue.is_empty() {
                    except_flag = true;
                    fdset[READ].set(fd);
                }
                if !fd.output.blocked && !fd.output.queue.is_empty() {
                    except_flag = true;
                    fdset[WRITE].set(fd);
                }
                if except_flag {
                    fdset[EXCEPT].set(fd);
                }
            }
        }

        schd.get_ready_timers(this);

        if {
            let mut nfds = -1;
            
            let rfds = if fdset[READ].max_fd() == INVALID_SOCKET {
                ptr::null_mut()
            } else {
                nfds = cmp::max(nfds, fdset[READ].max_fd() as i32);
                fdset[READ].as_raw()
            };

            let wfds = if fdset[WRITE].max_fd() == INVALID_SOCKET {
                ptr::null_mut()
            } else {
                nfds = cmp::max(nfds, fdset[WRITE].max_fd() as i32);
                fdset[WRITE].as_raw()
            };

            let efds = if fdset[EXCEPT].max_fd() == INVALID_SOCKET {
                ptr::null_mut()
            } else {
                nfds = cmp::max(nfds, fdset[EXCEPT].max_fd() as i32);
                fdset[EXCEPT].as_raw()
            };
            
            unsafe { select(nfds + 1, rfds, wfds, efds, &tv) }
        } > 0 {
            let this_len = this.len();
            {
                let mut fds = self.mutex.lock().unwrap();
                for fd in fds.iter_mut() {
                    (fd.dispatch)(fd, &fdset, this);
                }
            }
            self.outstanding_work.fetch_sub(this.len() - this_len, Ordering::SeqCst);
        }
    }

    pub fn register_intr_fd(&self, fd: &IntrFd) {
        let mut fds = self.mutex.lock().unwrap();
        fds.push(fd.0.clone());
    }

    pub fn deregister_intr_fd(&self, fd: &IntrFd) {
        let mut fds = self.mutex.lock().unwrap();
        let i = fds.iter().position(|e| e.as_raw_fd() == fd.as_raw_fd()).unwrap();
        fds.remove(i);
    }

    pub fn register_async_fd(&self, fd: &AsyncFd) {
        let mut fds = self.mutex.lock().unwrap();
        fds.push(fd.0.clone());
    }

    pub fn deregister_async_fd(&self, fd: &AsyncFd) {
        let mut fds = self.mutex.lock().unwrap();
        let i = fds.iter().position(|e| e.as_raw_fd() == fd.as_raw_fd()).unwrap();
        fds.remove(i);
    }

    pub fn cancel_all_fds(&self, this: &mut ThreadIoContext) {
        let mut fds = self.mutex.lock().unwrap();
        for fd in fds.iter_mut() {
            fd.clear_all(this, ECANCELED);
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

    fn next_op(&self, this: &mut ThreadIoContext, ops: &mut Ops) {
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

        let _fds = self.mutex.lock().unwrap();
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

fn dispatch_async(fd: &mut FdContext, fds: &[FdSet; 3], this: &mut ThreadIoContext) {
    if fds[EXCEPT].is_set(fd) {
        let ec = sock_error(fd);
        fd.clear_all(this, ec);
        return;
    }
    
    if fds[READ].is_set(fd) {
        fd.ready_input(this);
    }
    if fds[WRITE].is_set(fd) {
        fd.ready_output(this);
    }
}

fn dispatch_intr(fd: &mut FdContext, fds: &[FdSet; 3], _: &mut ThreadIoContext) {
    if fds[READ].is_set(fd) {
        let mut buf: [u8; 8] = unsafe { mem::uninitialized() };
        libc_ign!(read(fd, &mut buf));
    }
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
