use ffi::{SystemError, INVALID_ARGUMENT, Signal, OPERATION_CANCELED, RawFd, AsRawFd, close};
use core::{AsIoContext, IoContext, Perform, ThreadIoContext, Handle};

use std::io;
use std::mem;
use std::ptr;
use std::cell::{UnsafeCell};
use libc::{sigset_t, signalfd, sigemptyset, SFD_CLOEXEC, pthread_sigmask, sigaddset, sigdelset, SIG_BLOCK, sigismember, sigprocmask, sigfillset, SIG_SETMASK, SFD_NONBLOCK};

pub struct SignalImpl {
    ctx: IoContext,
    fd: Handle,
    mask: UnsafeCell<sigset_t>,
}

impl SignalImpl {
    pub fn new(ctx: &IoContext) -> Result<Box<Self>, SystemError> {
        let mut mask = unsafe { mem::uninitialized() };
        let soc = match unsafe {
            sigemptyset(&mut mask);
            signalfd(-1, &mask, SFD_CLOEXEC | SFD_NONBLOCK)
        } {
            -1 => return Err(SystemError::last_error()),
            fd => Box::new(SignalImpl {
                ctx: ctx.clone(),
                fd: Handle::socket(fd),
                mask: UnsafeCell::new(mask),
            }),
        };
        ctx.as_reactor().register_socket(&soc.fd);
        Ok(soc)
    }

    pub fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        this.as_ctx().clone().as_reactor().add_read_op(&self.fd, this, op, err)
    }

    pub fn next_read_op(&self, this: &mut ThreadIoContext) {
        this.as_ctx().clone().as_reactor().next_read_op(&self.fd, this)
    }

    pub fn cancel(&self) {
        self.ctx.clone().as_reactor().cancel_ops(&self.fd, &self.ctx, OPERATION_CANCELED)
    }

    pub fn add(&self, sig: Signal) -> Result<(), SystemError> {
        match unsafe {
            if sigismember(self.mask.get(), sig as i32) != 0 {
                 return Err(INVALID_ARGUMENT)
            }
            sigaddset(self.mask.get(), sig as i32);
            pthread_sigmask(SIG_BLOCK, self.mask.get(), ptr::null_mut());
            signalfd(self.fd.as_raw_fd(), self.mask.get(), 0)
        } {
            -1 => Err(SystemError::last_error()),
            _ => Ok(())
        }
    }

    pub fn remove(&self, sig: Signal) -> Result<(), SystemError> {
        match unsafe {
            if sigismember(self.mask.get(), sig as i32) == 0 {
                 return Err(INVALID_ARGUMENT)
            }
            sigdelset(self.mask.get(), sig as i32);
            signalfd(self.fd.as_raw_fd(), self.mask.get(), 0)
        } {
            -1 => Err(SystemError::last_error()),
            _ => Ok(())
        }
    }

    pub fn clear(&self) {
        unsafe {
            sigemptyset(self.mask.get());
            sigfillset(self.mask.get());
            sigprocmask(SIG_SETMASK, self.mask.get(), ptr::null_mut());
            signalfd(self.fd.as_raw_fd(), self.mask.get(), 0);
        }
    }
}

unsafe impl AsIoContext for SignalImpl {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.ctx) {
            this.as_ctx()
        } else {
            &self.ctx
        }
    }
}

impl Drop for SignalImpl {
    fn drop(&mut self) {
        self.clear();
        self.ctx.as_reactor().deregister_socket(&self.fd);
        close(self.fd.as_raw_fd());
    }
}

impl AsRawFd for SignalImpl {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}
