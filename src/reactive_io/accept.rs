use prelude::{Protocol, Endpoint, SockAddr};
use ffi::{self, INVALID_SOCKET};
use unsafe_cell::UnsafeRefCell;
use error::{ErrCode, READY, EINTR, EAGAIN, EWOULDBLOCK, ECANCELED, last_error};
use core::{IoContext, ThreadIoContext, Socket, workplace};
use async::{Receiver, Handler, WrappedHandler, Operation};
use reactive_io::{AsyncInput, getnonblock, setnonblock};

use std::io;
use std::marker::PhantomData;

pub fn accept<P, T, S>(soc: &T, pro: &P) -> io::Result<(S, P::Endpoint)>
    where P: Protocol,
          T: Socket<P>,
          S: Socket<P>,
{
    let mut ep = unsafe { pro.uninitialized() };
    let mut socklen = ep.capacity() as _;
    while !soc.as_ctx().stopped() {
        let acc = unsafe { ffi::accept(soc, &mut ep, &mut socklen) };
        if acc != INVALID_SOCKET {
            let acc = unsafe {
                ep.resize(socklen as usize);
                S::from_raw_fd(soc.as_ctx(), ep.protocol(), acc)
            };
            return Ok((acc, ep));
        }
        let ec = last_error();
        if ec != EINTR {
            return Err(ec.into());
        }
    }
    Err(ECANCELED.into())
}

struct AcceptHandler<T, P, S> { soc: UnsafeRefCell<T>, pro: P, _marker: PhantomData<S> }

impl<T, P, S> WrappedHandler<(S, P::Endpoint), io::Error> for AcceptHandler<T, P, S>
    where T: AsyncInput,
          P: Protocol,
          S: Socket<P>,
{
    fn perform(&mut self, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode, op: Operation<(S, P::Endpoint), io::Error, Self>) {
        let soc = unsafe { self.soc.as_ref() };
        match ec {
            READY => {
                let mut ep = unsafe { op.as_self().pro.uninitialized() };
                let mut socklen = ep.capacity() as _;
                let mode = getnonblock(soc).unwrap();
                setnonblock(soc, true).unwrap();

                while !ctx.stopped() {
                    let acc = unsafe { ffi::accept(soc, &mut ep, &mut socklen) };
                    if acc != INVALID_SOCKET {
                        setnonblock(soc, mode).unwrap();
                        let acc = unsafe { S::from_raw_fd(ctx, ep.protocol(), acc) };
                        soc.next_op(this);
                        op.send(ctx, Ok((acc, ep)));
                        return;
                    }

                    let ec = last_error();
                    if ec == EAGAIN || ec == EWOULDBLOCK {
                        setnonblock(soc, mode).unwrap();
                        soc.add_op(this, op, ec);
                        return;
                    }
                    if ec != EINTR {
                        setnonblock(soc, mode).unwrap();
                        soc.next_op(this);
                        op.send(ctx, Err(ec.into()));
                        return;
                    }
                }

                setnonblock(soc, mode).unwrap();
                soc.next_op(this);
                op.send(ctx, Err(ECANCELED.into()));
            },
            ec => {
                soc.next_op(this);
                op.send(ctx, Err(ec.into()));
            },
        }
    }
}

pub fn async_accept<T, P, S, F>(soc: &T, pro: P, handler: F) -> F::Output
    where T: AsyncInput,
          P: Protocol,
          S: Socket<P>,
          F: Handler<(S, P::Endpoint), io::Error>,
{
    let (op, res) = handler.channel(AcceptHandler {
        soc: UnsafeRefCell::new(soc),
        pro: pro,
        _marker: PhantomData
    });
    workplace(soc.as_ctx(), |this| soc.add_op(this, op, READY));
    res.recv(soc.as_ctx())
}
