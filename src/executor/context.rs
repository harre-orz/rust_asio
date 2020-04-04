//

use super::{Interrupter, Reactor, ReactorCallback};
use error::{ErrorCode};
use socket::{Blocking};
use socket_base::{NativeHandle, Protocol, Socket};

use context::{Context, Transfer};
use context::stack::{ProtectedFixedSizeStack, Stack, StackError};

use std::io;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{Ordering, AtomicUsize};
use std::collections::LinkedList;

enum Mode {
    Read, Write,
}

pub trait Wait {
    fn readable<P, S>(&mut self, soc: &S) -> ErrorCode
    where
        P: Protocol,
        S: Socket<P>;

    fn writable<P, S>(&mut self, soc: &S) -> ErrorCode
    where
        P: Protocol,
        S: Socket<P>;
}

pub struct YieldContext {
    ctx: IoContext,
    context: Option<Context>,
}

impl YieldContext {
    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    fn yield_call<P, S>(&mut self, soc: &S, mode: Mode) -> ErrorCode
    where
        P: Protocol,
        S: Socket<P>,
    {
        let context = self.context.take().unwrap();
        let data = (mode, soc.id());
        let Transfer { context, data } = unsafe { context.resume(&data as *const _ as _) };
        self.context = Some(context);
        ErrorCode::from_yield(data)
    }
}

impl Wait for YieldContext {
    fn readable<P, S>(&mut self, soc: &S) -> ErrorCode
        where P: Protocol,
              S: Socket<P>,
    {
        self.yield_call(soc, Mode::Read)
    }

     fn writable<P, S>(&mut self, soc: &S) -> ErrorCode
        where P: Protocol,
              S: Socket<P>,
    {
         self.yield_call(soc, Mode::Write)
    }
}

pub struct SocketContext {
    pub handle: NativeHandle,
    pub callback: ReactorCallback,
}

impl SocketContext {
    pub fn id(&self) -> usize {
        self as *const _ as _
    }
}

impl Drop for SocketContext {
    fn drop(&mut self) {
        let _ = unsafe { libc::close(self.handle) };
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
    stack: ProtectedFixedSizeStack,
    func: Box<dyn Exec>,
}

extern "C" fn entry(t: Transfer) -> ! {
    let Transfer { context, data } = t;
    let data = unsafe { &mut *(data as *mut Option<InitData>) };
    let InitData {
        ctx,
        stack,
        func,
    } = data.take().unwrap();
    let mut yield_ctx = YieldContext {
        ctx: ctx,
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

struct Inner {
    intr: Interrupter,
    reactor: Reactor,
    count: AtomicUsize,
    read_list: Mutex<LinkedList<(Context, usize)>>,
    write_list: Mutex<LinkedList<(Context, usize)>>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.intr.cleanup(&self.reactor);
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
        let intr = Interrupter::new()?;
        intr.startup(&reactor);
        Ok(IoContext {
            inner: Arc::new(Inner {
                intr: intr,
                reactor: reactor,
                count: AtomicUsize::new(0),
                read_list: Mutex::new(LinkedList::new()),
                write_list: Mutex::new(LinkedList::new()),
            }),
            block: Blocking::new(),
        })
    }

    pub(crate) fn blocking(&self) -> Blocking {
        self.block.clone()
    }

    pub(crate) fn register(&self, socket_ctx: &SocketContext) {
        self.inner.reactor.register_socket(socket_ctx)
    }

    pub(crate) fn deregister(&self, socket_ctx: &SocketContext) {
        self.inner.reactor.deregister_socket(socket_ctx)
    }

    pub fn is_stopped(&self) -> bool {
        false
    }

    pub fn run(&self) {
        while self.inner.count.load(Ordering::SeqCst) > 0 {
            self.inner.reactor.poll(self);
        }
    }

    fn yield_callback(&self, t: Transfer) {
        let Transfer { context, data } = t;
        if data == 0 {
            self.inner.count.fetch_sub(1, Ordering::SeqCst);
            return
        }

        let data = unsafe { &*(data as *const (Mode, usize)) };
        match data {
            &(Mode::Read, id) => {
                let mut list = self.inner.read_list.lock().unwrap();
                list.push_back((context, id));
            },
            &(Mode::Write, id) => {
                let mut list = self.inner.write_list.lock().unwrap();
                list.push_back((context, id))
            },
        }
    }

    pub(crate) fn read_callback(&self, socket_ctx: &SocketContext, ec: ErrorCode) {
        let mut left = LinkedList::new();
        let mut res = None;
        {
            let mut list = self.inner.read_list.lock().unwrap();
            while let Some(e) = list.pop_front() {
                if socket_ctx.id() == e.1 && res.is_none() {
                    res = Some(e.0)
                } else {
                    left.push_back(e);
                }
            }
            list.append(&mut left);
        }
        if let Some(context) = res.take() {
            self.yield_callback(unsafe { context.resume(ec.into_yield()) });
        }
    }

    pub(crate) fn write_callback(&self, socket_ctx: &SocketContext, ec: ErrorCode) {
        let mut left = LinkedList::new();
        let mut res = None;
        {
            let mut list = self.inner.write_list.lock().unwrap();
            while let Some(e) = list.pop_front() {
                if socket_ctx.id() == e.1 && res.is_none() {
                    res = Some(e.0)
                } else {
                    left.push_back(e);
                }
            }
            list.append(&mut left);
        }
        if let Some(context) = res.take() {
            self.yield_callback(unsafe { context.resume(ec.into_yield()) });
        }
    }

    pub fn spawn<F>(&self, func: F) -> Result<(), StackError>
    where
        F: FnOnce(&mut YieldContext) + 'static,
    {
        let init = InitData {
            ctx: self.clone(),
            stack: ProtectedFixedSizeStack::new(Stack::default_size())?,
            func: Box::new(func),
        };
        let context = unsafe { Context::new(&init.stack, entry) };
        self.inner.count.fetch_add(1, Ordering::SeqCst);
        let mut data = Some(init);
        self.yield_callback(unsafe { context.resume(&mut data as *mut _ as usize) });
        Ok(())
    }
}
