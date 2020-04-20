//

use super::{Intr, Reactor};
use error::{ErrorCode, TIMED_OUT};
use socket::{Blocking, close};
use socket_base::{NativeHandle, Socket};

use context::{Context, Transfer};
use context::stack::{ProtectedFixedSizeStack, Stack, StackError};

use std::io;
use std::sync::{Arc, Condvar, Mutex};
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

    pub fn dispatch(&mut self, io_ctx: &IoContext, ctx: Context, ec: ErrorCode) {
        if io_ctx.inner.thread_count.load(Ordering::SeqCst) > 1 {
            let mut list = io_ctx.inner.mutex.lock().unwrap();
            list.push_back((ctx, ec));
            io_ctx.inner.condvar.notify_one();
        } else {
            self.queue.push((ctx, ec))
        }
    }
}

struct Entry {
    context: Context,
    handle: NativeHandle,
    expire: Instant,
}

struct Inner {
    intr: Intr,
    reactor: Reactor,
    read_list: Mutex<LinkedList<Entry>>,
    write_list: Mutex<LinkedList<Entry>>,
    stopped: AtomicBool,
    coroutine_count: AtomicUsize,
    thread_count: AtomicUsize,
    mutex: Mutex<LinkedList<(Context, ErrorCode)>>,
    condvar: Condvar,
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
                read_list: Mutex::new(LinkedList::new()),
                write_list: Mutex::new(LinkedList::new()),
                stopped: AtomicBool::new(false),
                coroutine_count: AtomicUsize::new(0),
                thread_count: AtomicUsize::new(0),
                mutex: Mutex::new(LinkedList::new()),
                condvar: Condvar::new(),
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
        while self.inner.coroutine_count.load(Ordering::SeqCst) > 0 {
            if self.inner.thread_count.fetch_or(1, Ordering::SeqCst) & 1 == 0 {
                self.inner.reactor.poll(&self.inner.intr, self, &mut thrd_ctx);
                for (context, ec) in thrd_ctx.queue.drain(..) {
                    self.yield_callback(unsafe { context.resume(ec.into_yield()) })
                }
                self.inner.thread_count.fetch_sub(1, Ordering::SeqCst);
            } else {
                self.inner.thread_count.fetch_add(2, Ordering::SeqCst);
                if let Some((context, ec)) = {
                    let mut list = self.inner.mutex.lock().unwrap();
                    if let Some(e) = list.pop_front() {
                        Some(e)
                    } else {
                        list = self.inner.condvar.wait(list).unwrap();
                        list.pop_front()
                    }
                } {
                    self.yield_callback(unsafe { context.resume(ec.into_yield()) });
                }
                self.inner.thread_count.fetch_sub(2, Ordering::SeqCst);
            }
        }
    }

    fn yield_callback(&self, t: Transfer) {
        let Transfer { context, data } = t;
        if data == 0 {
            self.inner.coroutine_count.fetch_sub(1, Ordering::SeqCst);
            return
        }

        let mut temp = LinkedList::new();
        let data = unsafe { &*(data as *const (Mode, NativeHandle, Instant)) };
        match data {
            &(Mode::Read, handle, expire) => {
                self.inner.intr.reset_timeout(expire);
                let mut list = self.inner.read_list.lock().unwrap();
                while let Some(e) = list.pop_front() {
                    if e.expire > expire {
                        break
                    } else {
                        temp.push_back(e)
                    }

                }
                temp.push_back(Entry {
                    context: context,
                    handle: handle,
                    expire: expire
                });
                temp.append(&mut list);
                list.append(&mut temp);
            },
            &(Mode::Write, handle, expire) => {
                self.inner.intr.reset_timeout(expire);
                let mut list = self.inner.write_list.lock().unwrap();
                while let Some(e) = list.pop_front() {
                    if e.expire > expire {
                        break
                    } else {
                        temp.push_back(e)
                    }
                }
                temp.push_back(Entry {
                    context: context,
                    handle: handle,
                    expire: expire,
                });
                temp.append(&mut list);
                list.append(&mut temp);
            },
        }
    }

    pub(super) fn read_callback(&self, now: Instant, handle: NativeHandle, ec: ErrorCode, thrd_ctx: &mut ThreadContext) {
        let mut temp = LinkedList::new();
        let mut list = self.inner.read_list.lock().unwrap();
        while let Some(e) = list.pop_front() {
            if handle == e.handle {
                thrd_ctx.dispatch(self, e.context, ec);
                while let Some(e) = list.pop_front() {
                    if e.expire < now {
                        thrd_ctx.dispatch(self, e.context, TIMED_OUT)
                    } else {
                        temp.push_back(e)
                    }
                }
                break
            } else if e.expire < now {
                thrd_ctx.dispatch(self, e.context, TIMED_OUT)
            } else {
                temp.push_back(e)
            }
        }
        list.append(&mut temp)
    }

    pub(super) fn write_callback(&self, now: Instant, handle: NativeHandle, ec: ErrorCode, thrd_ctx: &mut ThreadContext) {
        let mut temp = LinkedList::new();
        let mut list = self.inner.write_list.lock().unwrap();
        while let Some(e) = list.pop_front() {
            if handle == e.handle {
                thrd_ctx.dispatch(self, e.context, ec);
                while let Some(e) = list.pop_front() {
                    if e.expire < now {
                        thrd_ctx.dispatch(self, e.context, TIMED_OUT)
                    } else {
                        temp.push_back(e)
                    }
                }
                break
            } else if e.expire < now {
                thrd_ctx.dispatch(self, e.context, TIMED_OUT)
            } else {
                temp.push_back(e)
            }
        }
        list.append(&mut temp)
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
        self.inner.coroutine_count.fetch_add(1, Ordering::SeqCst);
        let mut data = Some(init);
        self.yield_callback(unsafe { context.resume(&mut data as *mut _ as usize) });
        Ok(())
    }
}
