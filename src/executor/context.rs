//

use super::{Interrupter, Reactor, ReactorCallback, callback_interrupter, callback_socket};
use context_::stack::{ProtectedFixedSizeStack, Stack, StackError};
use context_::{Context, Transfer};
use error::{ErrorCode};
use socket::{Blocking};
use socket_base::{NativeHandle, Protocol, Socket};

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

    fn call<P, S>(&mut self, soc: &S, mode: Mode) -> ErrorCode
    where
        P: Protocol,
        S: Socket<P>,
    {
        let socket_ctx = soc.as_inner();
        let context = self.context.take().unwrap();
        let data = (socket_ctx, mode);
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
        self.call(soc, Mode::Read)
    }

     fn writable<P, S>(&mut self, soc: &S) -> ErrorCode
        where P: Protocol,
              S: Socket<P>,
    {
         self.call(soc, Mode::Write)
    }
}

pub struct SocketContext {
    handle: NativeHandle,
    pub callback: ReactorCallback,
}

impl SocketContext {
    pub fn interrupter(fd: NativeHandle) -> Self {
        SocketContext {
            handle: fd,
            callback: callback_interrupter,
        }
    }

    pub fn socket(fd: NativeHandle) -> Self {
        SocketContext {
            handle: fd,
            callback: callback_socket,
        }
    }

    pub fn register(&self, ctx: &IoContext) {
        ctx.inner.reactor.register_socket(self)
    }

    pub fn deregister(&self, ctx: &IoContext) {
        ctx.inner.reactor.deregister_socket(self)
    }

    pub fn native_handle(&self) -> NativeHandle {
        self.handle
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
    let stack = unsafe { &mut *(t.data as *mut Option<ProtectedFixedSizeStack>) };
    // Drop the stack
    let _ = stack.take().unwrap();
    t.data = 0;
    t
}

struct Inner {
    intr: Interrupter,
    reactor: Reactor,
    count: AtomicUsize,
    read_list: Mutex<LinkedList<(Context, *const SocketContext)>>,
    write_list: Mutex<LinkedList<(Context, *const SocketContext)>>,
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

    pub fn is_stopped(&self) -> bool {
        false
    }

    pub fn run(&self) {
        while self.inner.count.load(Ordering::SeqCst) > 0 {
            self.inner.reactor.poll(self);
        }
    }

    fn callback(&self, t: Transfer) {
        let Transfer { context, data } = t;
        if data == 0 {
            self.inner.count.fetch_sub(1, Ordering::SeqCst);
            return
        }

        let data = unsafe { &*(data as *const (&SocketContext, Mode)) };
        match data {
            (socket_ctx, Mode::Read) => {
                let mut list = self.inner.read_list.lock().unwrap();
                list.push_back((context, *socket_ctx));
            },
            (socket_ctx, Mode::Write) => {
                let mut list = self.inner.write_list.lock().unwrap();
                list.push_back((context, *socket_ctx))
            },
        }
    }

    pub(super) fn read_callback(&self, socket_ctx: &SocketContext, ec: ErrorCode) {
        use std::ptr;
        let mut left = LinkedList::new();
        let mut res = None;
        {
            let mut list = self.inner.read_list.lock().unwrap();
            while let Some(e) = list.pop_front() {
                println!("search callback {:p} = {:p}", e.1, socket_ctx);
                if ptr::eq(e.1, socket_ctx) && res.is_none() {
                    res = Some(e.0)
                } else {
                    left.push_back(e);
                }
            }
            list.append(&mut left);
        }
        if let Some(context) = res.take() {
            self.callback(unsafe { context.resume(ec.into_yield()) });
        }
    }

    pub(super) fn write_callback(&self, socket_ctx: &SocketContext, ec: ErrorCode) {
        use std::ptr;
        let mut left = LinkedList::new();
        let mut res = None;
        {
            let mut list = self.inner.write_list.lock().unwrap();
            while let Some(e) = list.pop_front() {
                if ptr::eq(e.1, socket_ctx) && res.is_none() {
                    res = Some(e.0)
                } else {
                    left.push_back(e);
                }
            }
            list.append(&mut left);
        }
        if let Some(context) = res.take() {
            self.callback(unsafe { context.resume(ec.into_yield()) });
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
        self.callback(unsafe { context.resume(&mut data as *mut _ as usize) });
        Ok(())
    }

    pub fn blocking(&self) -> Blocking {
        self.block.clone()
    }
}
