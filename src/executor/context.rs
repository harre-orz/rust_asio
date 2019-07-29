//

use super::{Interrupter, Reactor, ReactorCallback, TimerQueue};
use context_::stack::{ProtectedFixedSizeStack, Stack, StackError};
use context_::{Context, Transfer};
use error::{ErrorCode, TIMED_OUT, WOULD_BLOCK};
use socket_base::{NativeHandle, Protocol, Socket};
use std::io;
use std::ptr;
use std::sync::Arc;
use std::time::Instant;

trait Exec {
    fn call_box(self: Box<Self>, yield_ctx: &mut YieldContext);
}

impl<F> Exec for F
where
    F: FnOnce(&mut YieldContext),
{
    fn call_box(self: Box<Self>, yield_ctx: &mut YieldContext) {
        self(yield_ctx)
    }
}

pub trait Blocking {
    fn ready_reading<P, S>(&mut self, soc: &S) -> ErrorCode
        where P: Protocol,
              S: Socket<P>;

    fn ready_writing<P, S>(&mut self, soc: &S) -> ErrorCode
        where P: Protocol,
              S: Socket<P>;
}

pub struct YieldContext {
    io_ctx: IoContext,
    context: Option<Context>,
    pub expiry: Instant,
}

impl YieldContext {
    pub(super) fn consume(&mut self, reactor: &Reactor) {
        if let Some(context) = self.context.take() {
            reactor.mutex.unlock();
            callee(reactor, unsafe { context.resume(TIMED_OUT.into_yield()) });
            reactor.mutex.lock();
        }
    }
}

impl AsIoContext for YieldContext {
    fn as_ctx(&self) -> &IoContext {
        &self.io_ctx
    }
}

impl Blocking for YieldContext {
    fn ready_reading<P, S>(&mut self, soc: &S) -> ErrorCode
        where P: Protocol,
              S: Socket<P>
    {
        let socket_ctx = unsafe { &mut *(soc.as_inner() as *const _ as *mut SocketContext) };
        self.io_ctx.inner.reactor.mutex.lock(); // lock_A
        if socket_ctx.readable {
            self.io_ctx.inner.reactor.mutex.unlock(); // unlock_A
            WOULD_BLOCK
        } else {
            let inner = unsafe { &mut *(&self.io_ctx as *const _ as *mut Inner) };
            socket_ctx.yield_ctx = self;
            let context = self.context.take().unwrap();
            inner.timer_queue.insert(self, &mut inner.intr);
            let Transfer { context, data } = unsafe { context.resume(socket_ctx as *const _ as _) };
            self.io_ctx.inner.reactor.mutex.lock(); // lock_A
            inner.timer_queue.erase(self, &mut inner.intr);
            self.context = Some(context);
            self.io_ctx.inner.reactor.mutex.unlock(); // unlock_A
            ErrorCode::from_yield(data)
        }
    }

    fn ready_writing<P, S>(&mut self, soc: &S) -> ErrorCode
        where P: Protocol,
              S: Socket<P>,
    {
        let socket_ctx = unsafe { &mut *(soc.as_inner() as *const _ as *mut SocketContext) };
        self.io_ctx.inner.reactor.mutex.lock(); // lock_B
        if socket_ctx.writable {
            self.io_ctx.inner.reactor.mutex.unlock(); // unlock_B
            WOULD_BLOCK
        } else {
            let inner = unsafe { &mut *(&self.io_ctx as *const _ as *mut Inner) };
            socket_ctx.yield_ctx = self;
            let context = self.context.take().unwrap();
            inner.timer_queue.insert(self, &mut inner.intr);
            let Transfer { context, data } = unsafe { context.resume(socket_ctx as *const _ as _) };
            self.io_ctx.inner.reactor.mutex.lock(); // lock_B
            inner.timer_queue.erase(self, &mut inner.intr);
            self.context = Some(context);
            self.io_ctx.inner.reactor.mutex.unlock(); // unlock_B
            ErrorCode::from_yield(data)
        }
    }
}

struct InitData {
    io_ctx: IoContext,
    stack: ProtectedFixedSizeStack,
    func: Box<dyn Exec>,
}

extern "C" fn entry(t: Transfer) -> ! {
    let Transfer { context, data } = t;
    let data = unsafe { &mut *(data as *mut Option<InitData>) };
    let InitData {
        io_ctx,
        stack,
        func,
    } = data.take().unwrap();
    let mut yield_ctx = YieldContext {
        io_ctx: io_ctx,
        context: Some(context),
        expiry: Instant::now(),
    };
    func.call_box(&mut yield_ctx);
    let context = yield_ctx.context.take().unwrap();
    let mut stack = Some(stack);
    unsafe { context.resume_ontop(&mut stack as *mut _ as usize, exit) };
    unreachable!()
}

extern "C" fn exit(mut t: Transfer) -> Transfer {
    let stack = unsafe { &mut *(t.data as *mut Option<ProtectedFixedSizeStack>) };
    // Drop the stack
    let _ = stack.take().unwrap();
    t.data = 0;
    t
}

fn callee(reactor: &Reactor, t: Transfer) {
    let Transfer { context, data } = t;
    if data != 0 {
        let socket_ctx = unsafe { &mut *(data as *mut SocketContext) };
        let yield_ctx = unsafe { &mut *socket_ctx.yield_ctx };
        yield_ctx.context = Some(context);
        reactor.mutex.unlock(); // unlock_A, unlock_B
    }
}

pub struct SocketContext {
    yield_ctx: *mut YieldContext,
    readable: bool,
    writable: bool,
    handle: NativeHandle,
    pub callback: ReactorCallback,
}

impl SocketContext {
    pub fn interrupter(fd: NativeHandle) -> Self {
        SocketContext {
            yield_ctx: ptr::null_mut(),
            readable: false,
            writable: true,
            handle: fd,
            callback: Reactor::callback_interrupter,
        }
    }

    pub fn socket(fd: NativeHandle) -> Self {
        SocketContext {
            yield_ctx: ptr::null_mut(),
            readable: false,
            writable: true,
            handle: fd,
            callback: Reactor::callback_socket,
        }
    }

    pub fn register(&self, ctx: &IoContext) {}

    pub fn native_handle(&self) -> NativeHandle {
        self.handle
    }

    pub fn callback_readable(&mut self, reactor: &Reactor, data: ErrorCode) {
        reactor.mutex.lock(); // lock_C
        if self.yield_ctx.is_null() {
            self.readable = true;
            reactor.mutex.unlock(); // unlock_C
        } else {
            let yield_ctx = unsafe { &mut *self.yield_ctx };
            if let Some(context) = yield_ctx.context.take() {
                self.readable = false;
                reactor.mutex.unlock(); // unlock_C
                callee(reactor, unsafe { context.resume(data.into_yield()) });
            } else {
                // timed out
                self.readable = true;
                reactor.mutex.unlock(); // unlock_C
            }
        }
    }

    pub fn callback_writable(&mut self, reactor: &Reactor, data: ErrorCode) {
        reactor.mutex.lock(); // lock_D
        if self.yield_ctx.is_null() {
            self.writable = true;
            reactor.mutex.unlock(); // unlock_D
        } else {
            let yield_ctx = unsafe { &mut *self.yield_ctx };
            if let Some(context) = yield_ctx.context.take() {
                self.writable = false;
                reactor.mutex.unlock(); // unlock_D
                callee(reactor, unsafe { context.resume(data.into_yield()) })
            } else {
                // timed out
                self.writable = true;
                reactor.mutex.unlock(); // unlock_D
            }
        }
    }
}

impl Drop for SocketContext {
    fn drop(&mut self) {
        let _ = unsafe { libc::close(self.handle) };
    }
}

struct Inner {
    intr: Interrupter,
    reactor: Reactor,
    timer_queue: TimerQueue,
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.intr.cleanup(&self.reactor);
    }
}

#[derive(Clone)]
pub struct IoContext {
    inner: Arc<Inner>,
}

impl IoContext {
    pub fn new() -> io::Result<Self> {
        let reactor = Reactor::new()?;
        let intr = Interrupter::new()?;
        intr.startup(&reactor);
        Ok(IoContext {
            inner: Arc::new(Inner {
                reactor: reactor,
                intr: intr,
                timer_queue: TimerQueue::new(),
            }),
        })
    }

    pub fn is_stopped(&self) -> bool {
        false
    }

    pub fn run(&self) {
        // FIXME
        let timer_queue = unsafe { &mut *(&self.inner.timer_queue as *const _ as *mut TimerQueue) };
        self.inner
            .reactor
            .poll(timer_queue, self.inner.intr.wait_duration(100) as i32);
    }

    pub fn spawn<F>(&self, func: F) -> Result<(), StackError>
    where
        F: FnOnce(&mut YieldContext) + 'static,
    {
        let init = InitData {
            io_ctx: self.clone(),
            stack: ProtectedFixedSizeStack::new(Stack::default_size())?,
            func: Box::new(func),
        };
        let context = unsafe { Context::new(&init.stack, entry) };
        let mut data = Some(init);
        let t = unsafe { context.resume(&mut data as *mut _ as usize) };
        Ok(callee(&self.inner.reactor, t))
    }
}

pub trait AsIoContext {
    fn as_ctx(&self) -> &IoContext;
}

impl AsIoContext for IoContext {
    fn as_ctx(&self) -> &IoContext {
        self
    }
}
