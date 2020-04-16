//

use super::{Intr, Reactor};
use error::{ErrorCode};
use socket::{Blocking, close};
use socket_base::{NativeHandle, Socket};

use context::{Context, Transfer};
use context::stack::{ProtectedFixedSizeStack, Stack, StackError};

use std::io;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{Ordering, AtomicBool, AtomicUsize};
use std::collections::LinkedList;
use std::time::{Instant, Duration};
use std::mem::MaybeUninit;

enum Mode {
    Read, Write,
}

pub trait Wait {
    fn readable<P, S>(&mut self, soc: &S) -> ErrorCode
    where
        S: Socket<P>;

    fn writable<P, S>(&mut self, soc: &S) -> ErrorCode
    where
        S: Socket<P>;
}

fn infinit() -> Instant {
    let unit: Instant = unsafe { MaybeUninit::zeroed().assume_init() };
    unit + Duration::new(60 * 60 * 24 * 365 * 100, 0)
}

pub struct YieldContext {
    ctx: IoContext,
    expire: Instant,
    context: Option<Context>,
}

impl YieldContext {
    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn expires_at(&mut self, expire: Instant) {
        self.expire = expire
    }

    pub fn cancel(&mut self) {
        self.expire = infinit()
    }

    fn yield_call<P, S>(&mut self, soc: &S, mode: Mode) -> ErrorCode
    where
        S: Socket<P>,
    {
        let context = self.context.take().unwrap();
        let data = (mode, soc.native_handle(), self.expire);
        let Transfer { context, data } = unsafe { context.resume(&data as *const _ as _) };
        self.context = Some(context);
        ErrorCode::from_yield(data)
    }
}

impl Wait for YieldContext {
    fn readable<P, S>(&mut self, soc: &S) -> ErrorCode
        where S: Socket<P>,
    {
        self.yield_call(soc, Mode::Read)
    }

     fn writable<P, S>(&mut self, soc: &S) -> ErrorCode
        where S: Socket<P>,
    {
         self.yield_call(soc, Mode::Write)
    }
}

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

struct InitData {
    ctx: IoContext,
    func: Box<dyn Exec>,
    stack: ProtectedFixedSizeStack,
}

extern "C" fn entry(t: Transfer) -> ! {
    let Transfer { context, data } = t;
    let data = unsafe { &mut *(data as *mut Option<InitData>) };
    let InitData {
        ctx,
        func,
        stack,
    } = data.take().unwrap();
    let mut yield_ctx = YieldContext {
        ctx: ctx,
        expire: infinit(),
        context: Some(context),
    };
    func.call_box(&mut yield_ctx);
    let context = yield_ctx.context.take().unwrap();
    let mut stack = Some(stack);
    unsafe { context.resume_ontop(&mut stack as *mut _ as usize, exit) };
    unreachable!()
}

extern "C" fn exit(mut t: Transfer) -> Transfer {
    use std::mem;

    let stack = unsafe { &mut *(t.data as *mut Option<ProtectedFixedSizeStack>) };
    mem::forget(stack.take().unwrap());
    t.data = 0;
    t
}

pub struct ThreadContext {
    queue: Vec<(Context, ErrorCode)>,
}

impl ThreadContext {
    pub fn new() -> Self {
        ThreadContext {
            queue: Vec::new(),
        }
    }

    pub fn push(&mut self, ctx: Context, ec: ErrorCode) {
        self.queue.push((ctx, ec))
    }
}

struct Inner {
    intr: Intr,
    reactor: Reactor,
    count: AtomicUsize,
    read_list: Mutex<LinkedList<(NativeHandle, Instant, Context)>>,
    write_list: Mutex<LinkedList<(NativeHandle, Instant, Context)>>,
    stopped: AtomicBool,
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.reactor.deregister_intr(&self.intr);
    }
}

#[derive(Clone)]
pub struct IoContext {
    inner: Arc<Inner>,
    block: Blocking,
}

impl IoContext {
    pub fn new() -> io::Result<Self> {
        let reactor = Reactor::new()?;
        let intr = Intr::new()?;
        reactor.register_intr(&intr);
        Ok(IoContext {
            inner: Arc::new(Inner {
                intr: intr,
                reactor: reactor,
                count: AtomicUsize::new(0),
                read_list: Mutex::new(LinkedList::new()),
                write_list: Mutex::new(LinkedList::new()),
                stopped: AtomicBool::new(false),
            }),
            block: Blocking::infinit(),
        })
    }

    pub(crate) fn blocking(&self) -> Blocking {
        self.block.clone()
    }

    pub(crate) fn placement<P, S>(&self, soc: S) -> S
        where S: Socket<P>
    {
        self.inner.reactor.register_socket(&soc);
        soc
    }

    pub(crate) fn disposal<P, S>(&self, soc: &S) -> Result<(), ErrorCode>
        where S: Socket<P>
    {
        self.inner.reactor.deregister_socket(soc);
        close(soc.native_handle())
    }

    pub fn expires_after(&mut self, expire: Duration) {
        self.block.expires_after(expire)
    }

    pub fn is_stopped(&self) -> bool {
        self.inner.stopped.load(Ordering::SeqCst)
    }

    pub fn stop(&self) -> bool{
        if self.inner.stopped.fetch_or(true, Ordering::SeqCst) {
            return false
        }
        self.inner.intr.interrupt();
        return true
    }

    pub fn run(&self) {
        let mut thrd_ctx = ThreadContext::new();
        while self.inner.count.load(Ordering::SeqCst) > 0 {
            self.inner.reactor.poll(&self.inner.intr, self, &mut thrd_ctx);
            for (context, ec) in thrd_ctx.queue.drain(..) {
                self.yield_callback(unsafe { context.resume(ec.into_yield()) });
            }
        }
    }

    fn yield_callback(&self, t: Transfer) {
        let Transfer { context, data } = t;
        if data == 0 {
            self.inner.count.fetch_sub(1, Ordering::SeqCst);
            return
        }

        let data = unsafe { &*(data as *const (Mode, NativeHandle, Instant)) };
        match data {
            &(Mode::Read, handle, expire) => {
                let mut list = self.inner.read_list.lock().unwrap();
                list.push_back((handle, expire, context));
            },
            &(Mode::Write, handle, expire) => {
                let mut list = self.inner.write_list.lock().unwrap();
                list.push_back((handle, expire, context))
            },
        }
    }

    pub(super) fn read_callback(&self, handle: NativeHandle, ec: ErrorCode, thrd_ctx: &mut ThreadContext) {
        let mut left = LinkedList::new();
        let mut list = self.inner.read_list.lock().unwrap();
        while let Some(e) = list.pop_front() {
            if handle == e.0 {
                thrd_ctx.push(e.2, ec);
                left.append(&mut list);
                break
            } else {
                left.push_back(e);
            }
        }
        list.append(&mut left);
    }

    pub(super) fn write_callback(&self, handle: NativeHandle, ec: ErrorCode, thrd_ctx: &mut ThreadContext) {
        let mut left = LinkedList::new();
        let mut list = self.inner.write_list.lock().unwrap();
        while let Some(e) = list.pop_front() {
            if handle == e.0 {
                thrd_ctx.push(e.2, ec);
                left.append(&mut list);
            } else {
                left.push_back(e);
            }
        }
        list.append(&mut left);
    }

    pub fn spawn<F>(&self, func: F) -> Result<(), StackError>
    where
        F: FnOnce(&mut YieldContext) + 'static,
    {
        let init = InitData {
            ctx: self.clone(),
            func: Box::new(func),
            stack: ProtectedFixedSizeStack::new(Stack::default_size())?,
        };
        let context = unsafe { Context::new(&init.stack, entry) };
        self.inner.count.fetch_add(1, Ordering::SeqCst);
        let mut data = Some(init);
        self.yield_callback(unsafe { context.resume(&mut data as *mut _ as usize) });
        Ok(())
    }
}
