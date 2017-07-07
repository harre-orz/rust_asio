use lazy_box::LazyBox;
use ffi::{RawFd, AsRawFd, close};
use error::{ErrCode, READY};
use core::{IoContext, AsIoContext, Dispatch, Operation, ThreadIoContext};

use std::io;
use std::mem;
use std::ops::Deref;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};

pub trait Dispatcher {
    fn dispatcher() -> Dispatch;
}

#[derive(Default)]
pub struct Ops {
    pub queue: VecDeque<Operation>,
    pub blocked: bool,
    pub canceled: bool,
}

#[cfg(not(windows))]
#[derive(Default)]
pub struct FdContextExt;

#[cfg(windows)]
#[derive(Default)]
pub struct FdContextExt {
    nonblocking: ::std::cell::Cell<bool>,
}

pub struct FdContext {
    fd: RawFd,
    ctx: IoContext,
    pub input: Ops,
    pub output: Ops,
    pub dispatch: Dispatch,
    ext: FdContextExt,
}

impl FdContext {
    pub fn clear_all(&mut self, this: &mut ThreadIoContext, ec: ErrCode) -> usize {
        let len = self.input.queue.len() + self.output.queue.len();
        self.input.blocked = false;
        self.input.canceled = false;
        for op in self.input.queue.drain(..) {
            this.push(op, ec);
        }
        self.output.blocked = false;
        self.output.canceled = false;
        for op in self.output.queue.drain(..) {
            this.push(op, ec);
        }
        len
    }

    pub fn ready_input(&mut self, this: &mut ThreadIoContext) -> usize {
        if let Some(op) = self.input.queue.pop_front() {
            self.input.blocked = true;
            this.push(op, READY);
            1
        } else {
            self.input.blocked = false;
            0
        }
    }

    pub fn ready_output(&mut self, this: &mut ThreadIoContext) -> usize {
        if let Some(op) = self.output.queue.pop_front() {
            self.output.blocked = true;
            this.push(op, READY);
            1
        } else {
            self.output.blocked = false;
            0
        }
    }
}

impl FdContext {
    fn forget_ctx(self) {
        mem::forget(self.ctx)
    }
}

impl AsRawFd for FdContext {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Eq for FdContext {
}

impl PartialEq for FdContext {
    fn eq(&self, other: &Self) -> bool {
        (self as *const _) == (other as *const _)
    }
}

impl Hash for FdContext {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self as *const _ as usize);
    }
}

pub struct IntrFd(pub LazyBox<FdContext>);

impl Drop for IntrFd {
    fn drop(&mut self) {
        close(self.0.fd);
        self.0.release().forget_ctx();
    }
}

impl IntrFd {
    pub fn new<T: Dispatcher>(fd: RawFd) -> Self {
        IntrFd(LazyBox::new(FdContext {
            fd: fd,
            ctx: unsafe { mem::uninitialized() },
            input: Default::default(),
            output: Default::default(),
            dispatch: T::dispatcher(),
            ext: Default::default(),
        }))
    }
}

impl AsRawFd for IntrFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0.fd
    }
}

impl Deref for IntrFd {
    type Target = FdContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct AsyncFd(pub LazyBox<FdContext>);

impl Drop for AsyncFd {
    fn drop(&mut self) {
        self.0.ctx.0.reactor.deregister_async_fd(self);
        close(self.0.fd);
        self.0.release();
    }
}

impl AsyncFd {
    pub fn new<T: Dispatcher>(fd: RawFd, ctx: &IoContext) -> Self {
        let fd = AsyncFd(LazyBox::new(FdContext {
            fd: fd,
            ctx: ctx.clone(),
            input: Default::default(),
            output: Default::default(),
            dispatch: T::dispatcher(),
            ext: Default::default(),
        }));
        ctx.0.reactor.register_async_fd(&fd);
        fd
    }

    #[cfg(not(windows))]
    pub fn getnonblock(&self) -> io::Result<bool> {
        use ffi::getnonblock;
        getnonblock(self)
    }

    #[cfg(not(windows))]
    pub fn setnonblock(&self, on: bool) -> io::Result<()> {
        use ffi::setnonblock;
        setnonblock(self, on)
    }

    #[cfg(windows)]
    pub fn getnonblock(&self) -> io::Result<bool> {
        Ok(self.ext.nonblocking.get())
    }

    #[cfg(windows)]
    pub fn setnonblock(&self, on: bool) -> io::Result<()> {
        use ffi::ioctl;
        use socket_base::NonBlockingIo;
        self.ext.nonblocking.set(on);
        ioctl(self, &mut NonBlockingIo::new(on))
    }
}

impl AsRawFd for AsyncFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0.fd
    }
}

unsafe impl AsIoContext for AsyncFd {
    fn as_ctx(&self) -> &IoContext {
        &self.0.ctx
    }
}

impl Deref for AsyncFd {
    type Target = FdContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
