use prelude::{Protocol, SockAddr, Endpoint};
use ffi::{self, AsRawFd, socket};
use unsafe_cell::UnsafeRefCell;
use error::{READY, EINPROGRESS, EWOULDBLOCK, EINTR, ECANCELED,
            ErrCode, last_error, host_not_found};
use core::{IoContext, ThreadIoContext, Socket, workplace};
use async::{Receiver, Handler, WrappedHandler, Operation};
use reactive_io::{AsyncOutput, getnonblock, setnonblock};

use std::io;
use std::marker::PhantomData;

#[cfg(not(target_os = "macos"))]
fn connection_check<T>(_: &T) -> io::Result<()>
    where T: AsRawFd,
{
    Ok(())
}

#[cfg(target_os = "macos")]
fn connection_check<T>(soc: &T) -> io::Result<()>
    where T: AsRawFd,
{
    let mut buf = [0; 0];
    libc_try!(ffi::read(soc, &mut buf));
    Ok(())
}

pub fn connect<P, T, E>(soc: &T, ep: &E) -> io::Result<()>
    where P: Protocol,
          T: Socket<P>,
          E: SockAddr,
{
    while !soc.as_ctx().stopped() {
        if unsafe { ffi::connect(soc, ep) } == 0 {
            return connection_check(soc);
        }
        let ec = last_error();
        if ec != EINTR {
            return Err(ec.into());
        }
    }
    Err(ECANCELED.into())
}

pub struct ConnectHandler<T, E> { soc: UnsafeRefCell<T>, ep: E, mode: bool }

impl<T, E> WrappedHandler<(), io::Error> for ConnectHandler<T, E>
    where T: AsyncOutput,
          E: SockAddr,
{
    fn perform(&mut self, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode, op: Operation<(), io::Error, Self>) {
        let soc = unsafe { self.soc.as_ref() };
        match ec {
            READY => {
                let res = connection_check(soc);
                setnonblock(soc, self.mode).unwrap();
                soc.next_op(this);
                op.send(ctx, res)
            },
            ec => {
                setnonblock(soc, self.mode).unwrap();
                soc.next_op(this);
                op.send(ctx, Err(ec.into()))
            },
        }
    }
}

pub fn async_connect<T, E, F>(soc: &T, ep: &E, handler: F) -> F::Output
    where T: AsyncOutput,
          E: SockAddr,
          F: Handler<(), io::Error>,
{
    let mode = getnonblock(soc).unwrap();
    setnonblock(soc, true).unwrap();

    while !soc.as_ctx().stopped() {

        if unsafe { ffi::connect(soc, ep) } == 0 {
            setnonblock(soc, mode).unwrap();
            return handler.result(soc.as_ctx(), Ok(()));
        }

        let ec = last_error();
        if ec == EINPROGRESS || ec == EWOULDBLOCK {
            let (op, res) = handler.channel(ConnectHandler {
                soc: UnsafeRefCell::new(soc),
                ep: ep.clone(),
                mode: mode,
            });
            workplace(soc.as_ctx(), |this| soc.add_op(this, op, EINPROGRESS));
            return res.recv(soc.as_ctx());
        }
        if ec != EINTR {
            setnonblock(soc, mode).unwrap();
            return handler.result(soc.as_ctx(), Err(ec.into()));
        }
    }

    setnonblock(soc, mode).unwrap();
    return handler.result(soc.as_ctx(), Ok(()));
}

struct ConnectIterHandler<P, E, I, S> { res: Option<(S, E)>, it: I, _marker: PhantomData<P> }

impl<P, E, I, S> WrappedHandler<(S, E), io::Error> for ConnectIterHandler<P, E, I, S,>
    where P: Protocol,
          E: Endpoint<P>,
          I: Iterator<Item = E>,
          S: Socket<P> + AsyncOutput,
{
    fn perform(&mut self, ctx: &IoContext, this: &mut ThreadIoContext, ec: ErrCode, op: Operation<(S, E), io::Error, Self>) {
        let res = self.res.take().unwrap();
        let ep = match ec {
            READY => {
                op.send(ctx, Ok(res));
                return;
            },
            ECANCELED => {
                op.send(ctx, Err(ECANCELED.into())); // TODO: 明示的なキャンセルは未実装
                return;
            },
            _ => match self.it.next() {
                None => {
                    op.send(ctx, Err(host_not_found()));
                    return;
                },
                Some(ep) => ep,
            }
        };

        ::std::mem::drop(res);  // 先にソケットを解放しておく
        let pro = ep.protocol();
        match socket(&pro) {
            Ok(soc) => {
                let soc = unsafe { S::from_raw_fd(ctx, pro, soc) };
                while !ctx.stopped() {
                    setnonblock(&soc, true).unwrap();
                    if unsafe { ffi::connect(&soc, &ep) } == 0 {
                        setnonblock(&soc, false).unwrap();
                        return op.send(ctx, Ok((soc, ep)));
                    }

                    let ec = last_error();
                    if ec == EINPROGRESS || ec == EWOULDBLOCK {
                        let soc = {
                            self.res = Some((soc, ep));  // ここで、ヒープに移動
                            UnsafeRefCell::new(&self.res.as_mut().unwrap().0)
                        };
                        unsafe { soc.as_ref() }.add_op(this, op, READY);
                        return;
                    }
                    if ec != EINTR {
                        op.send(ctx, Err(ec.into()));
                        return;
                    }
                }
                op.send(ctx, Err(ECANCELED.into()))
            },
            Err(err) => op.send(ctx, Err(err)),
        }
    }
}

pub fn async_connect_iterator<P, E, I, S, F>(ctx: &IoContext, mut it: I, handler: F) -> F::Output
    where P: Protocol,
          E: Endpoint<P>,
          I: Iterator<Item = E> + Send + 'static,
          S: Socket<P> + AsyncOutput,
          F: Handler<(S, E), io::Error>,
{
    match it.next() {
        Some(ep) => {
            let pro = ep.protocol();
            match socket(&pro) {
                Ok(soc) => {
                    let soc = unsafe { S::from_raw_fd(ctx, pro, soc) };
                    while !ctx.stopped() {
                        setnonblock(&soc, true).unwrap();
                        if unsafe { ffi::connect(&soc, &ep) } == 0 {
                            setnonblock(&soc, false).unwrap();
                            return handler.result(ctx, Ok((soc, ep)));
                        }

                        let ec = last_error();
                        if ec == EINPROGRESS || ec == EWOULDBLOCK {
                            let (op, res) = handler.channel(ConnectIterHandler {
                                res: Some((soc, ep)),
                                it: it,
                                _marker: PhantomData,
                            });
                            let soc = UnsafeRefCell::new(&op.as_self().res.as_ref().unwrap().0);
                            workplace(ctx, |this| unsafe { soc.as_ref() }.add_op(this, op, READY));
                            return res.recv(ctx);
                        }
                        if ec != EINTR {
                            return handler.result(ctx, Err(ec.into()));
                        }
                    }
                    handler.result(ctx, Err(ECANCELED.into()))
                },
                Err(err) => handler.result(ctx, Err(err)),
            }
        },
        None => handler.result(ctx, Err(host_not_found())),
    }
}
