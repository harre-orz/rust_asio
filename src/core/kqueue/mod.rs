use super::{IoContext, AsIoContext, ThreadIoContext, Task, Perform};
use prelude::{Protocol, Socket};
use ffi::{RawFd, AsRawFd, SystemError};

pub struct KqueueReactor;


pub struct KqueueSocket<P> {
    ctx: IoContext,
    soc: RawFd,
    pro: P,
}

impl<P> KqueueSocket<P> {
    pub fn new(ctx: &IoContext, soc: RawFd, pro: P) -> Box<Self> {
        box KqueueSocket {
            pro: pro,
            ctx: ctx.clone(),
            soc: soc,
        }
    }
}

unsafe impl<P> AsIoContext for KqueueSocket<P> {
    fn as_ctx(&self) -> &IoContext {
        if let Some(this) = ThreadIoContext::callstack(&self.ctx) {
            this.as_ctx()
        } else {
            &self.ctx
        }
    }
}

impl<P: Protocol> Socket<P> for KqueueSocket<P> {
    fn protocol(&self) -> &P {
        &self.pro
    }
}

impl<P> AsRawFd for KqueueSocket<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.soc
    }
}

impl<P> KqueueSocket<P> {
    pub fn register_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        //this.push(op)
    }
}

// use ffi::{RawFd, AsRawFd};
// use core::{IoContext, AsIoContext, ThreadIoContext, Task};
//
// use std::collections::VecDeque;
// use errno::Errno;
//
// #[derive(Default)]
// pub struct Ops {
//     pub queue: VecDeque<Box<Operation>>
// }
//
//
// pub struct KqueueSocket<M> {
//     ctx: IoContext,
//     soc: RawFd,
//     pub read: Ops,
//     pub write: Ops,
//     pub mode: M
// }
//
// impl<M: Default> KqueueSocket<M> {
//     pub fn new(ctx: &IoContext, soc: RawFd) -> Box<Self> {
//         Box::new(KqueueSocket {
//             ctx: ctx.clone(),
//             soc: soc,
//             read: Default::default(),
//             write: Default::default(),
//             mode: Default::default(),
//         })
//     }
//
//     pub fn register_read_op(&self, this: &mut ThreadIoContext, op: Box<Operation>, errno: Errno) {
//     }
// }
//
// unsafe impl<M> AsIoContext for KqueueSocket<M> {
//     fn as_ctx(&self) -> &IoContext {
//         &self.ctx
//     }
// }
//
//
// impl<M> AsRawFd for KqueueSocket<M> {
//     fn as_raw_fd(&self) -> RawFd {
//         self.soc
//     }
// }
//
// pub struct KqueueReactor {
// }
