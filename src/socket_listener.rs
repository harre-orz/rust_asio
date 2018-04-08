#![allow(unreachable_patterns)]

use ffi::{AsRawFd, RawFd, SystemError, Timeout, socket, bind, listen, ioctl, getsockopt, setsockopt,
          getsockname};
use core::{Protocol, Socket, IoControl, GetSocketOption, SetSocketOption, AsIoContext, SocketImpl,
           IoContext, Perform, ThreadIoContext, Cancel, TimeoutLoc};
use handler::{Handler, AsyncReadOp};
use socket_base::{MAX_CONNECTIONS};

use std::io;
use std::fmt;
use std::time::Duration;

pub struct SocketListener<P> {
    pimpl: Box<SocketImpl<P>>,
}

impl<P> SocketListener<P>
where
    P: Protocol,
{
    pub fn new(ctx: &IoContext, pro: P) -> io::Result<Self> {
        let soc = socket(&pro)?;
        Ok(unsafe { Self::from_raw_fd(ctx, soc, pro) })
    }

    pub fn accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(blocking_accept(self, &self.pimpl.read_timeout)?)
    }

    pub fn async_accept<F>(&self, handler: F) -> F::Output
    where
        F: Handler<(P::Socket, P::Endpoint), io::Error>,
    {
        async_accept(self, handler)
    }

    pub fn bind(&self, ep: &P::Endpoint) -> io::Result<()> {
        Ok(bind(self, ep)?)
    }

    pub fn listen(&self) -> io::Result<()> {
        Ok(listen(self, MAX_CONNECTIONS)?)
    }

    pub fn local_endpoint(&self) -> io::Result<P::Endpoint> {
        Ok(getsockname(self)?)
    }

    pub fn nonblicking_accept(&self) -> io::Result<(P::Socket, P::Endpoint)> {
        Ok(nonblocking_accept(self)?)
    }

    pub fn get_accept_timeout(&self) -> Duration {
        self.pimpl.read_timeout.get()
    }

    pub fn get_option<C>(&self) -> io::Result<C>
    where
        C: GetSocketOption<P>,
    {
        Ok(getsockopt(self)?)
    }

    pub fn io_control<C>(&self, cmd: &mut C) -> io::Result<()>
    where
        C: IoControl,
    {
        Ok(ioctl(self, cmd)?)
    }

    pub fn set_accept_timeout(&self, timeout: Duration) -> io::Result<()> {
        Ok(self.pimpl.read_timeout.set(timeout)?)
    }

    pub fn set_option<C>(&self, cmd: C) -> io::Result<()>
    where
        C: SetSocketOption<P>,
    {
        Ok(setsockopt(self, cmd)?)
    }
}

unsafe impl<P> AsIoContext for SocketListener<P> {
    fn as_ctx(&self) -> &IoContext {
        self.pimpl.as_ctx()
    }
}

impl<P> Cancel for SocketListener<P> {
    fn cancel(&self) {
        self.pimpl.cancel()
    }

    fn as_timeout(&self, loc: TimeoutLoc) -> &Timeout {
        self.pimpl.as_timeout(loc)
    }
}

impl<P> AsRawFd for SocketListener<P> {
    fn as_raw_fd(&self) -> RawFd {
        self.pimpl.as_raw_fd()
    }
}

impl<P> AsyncReadOp for SocketListener<P>
where
    P: Protocol,
{
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError) {
        self.pimpl.add_read_op(this, op, err)
    }

    fn next_read_op(&self, this: &mut ThreadIoContext) {
        self.pimpl.next_read_op(this)
    }
}

impl<P> fmt::Debug for SocketListener<P>
where
    P: Protocol + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({})", self.protocol(), self.as_raw_fd())
    }
}

unsafe impl<P> Send for SocketListener<P> {}

unsafe impl<P> Sync for SocketListener<P> {}

impl<P> Socket<P> for SocketListener<P>
where
    P: Protocol,
{
    fn protocol(&self) -> &P {
        &self.pimpl.data
    }

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self {
        SocketListener { pimpl: SocketImpl::new(ctx, soc, pro) }
    }
}

use self::ops::{async_accept, blocking_accept, nonblocking_accept};
mod ops {
    use ffi::{SystemError, Timeout, accept, readable, OPERATION_CANCELED, TRY_AGAIN, WOULD_BLOCK,
              INTERRUPTED};
    use core::{Protocol, Socket, AsIoContext, Perform, Exec, ThreadIoContext, Cancel, TimeoutLoc};
    use handler::{Handler, Complete, Yield, NoYield, AsyncReadOp, Failure};

    use std::io;
    use std::marker::PhantomData;

    struct AsyncAccept<P, S, F> {
        soc: *const S,
        handler: F,
        _marker: PhantomData<P>,
    }

    unsafe impl<P, S, F> Send for AsyncAccept<P, S, F> {}

    impl<P, S, F> Handler<(P::Socket, P::Endpoint), io::Error> for AsyncAccept<P, S, F>
        where
        P: Protocol,
        S: Socket<P> + AsyncReadOp + Cancel,
        F: Complete<(P::Socket, P::Endpoint), io::Error>,
    {
        type Output = ();

        type Caller = Self;

        type Callee = NoYield;

        fn channel(self) -> (Self::Caller, Self::Callee) {
            (self, NoYield)
        }
    }

    impl<P, S, F> Complete<(P::Socket, P::Endpoint), io::Error> for AsyncAccept<P, S, F>
        where
        P: Protocol,
        S: Socket<P> + AsyncReadOp + Cancel,
        F: Complete<(P::Socket, P::Endpoint), io::Error>,
    {
        fn success(self, this: &mut ThreadIoContext, res: (P::Socket, P::Endpoint)) {
            let soc = unsafe { &*self.soc };
            soc.next_read_op(this);
            self.handler.success(this, res)
        }

        fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
            let soc = unsafe { &*self.soc };
            soc.next_read_op(this);
            self.handler.failure(this, err)
        }
    }

    impl<P, S, F> Perform for AsyncAccept<P, S, F>
    where
        P: Protocol,
        S: Socket<P> + AsyncReadOp + Cancel,
        F: Complete<(P::Socket, P::Endpoint), io::Error>,
    {
        fn perform(self: Box<Self>, this: &mut ThreadIoContext, err: SystemError) {
            let soc = unsafe { &*self.soc };
            if err != Default::default() {
                return self.failure(this, err.into());
            }

            loop {
                match accept(soc) {
                    Ok((acc, ep)) => {
                        let pro = soc.protocol().clone();
                        let soc = unsafe { P::Socket::from_raw_fd(this.as_ctx(), acc, pro) };
                        return self.success(this, (soc, ep));
                    }
                    Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                        return soc.add_read_op(this, self, WOULD_BLOCK)
                    }
                    Err(INTERRUPTED) if !soc.as_ctx().stopped() => {}
                    Err(err) => return self.failure(this, err.into()),
                }
            }
        }
    }

    impl<P, S, F> Exec for AsyncAccept<P, S, F>
    where
        P: Protocol,
        S: Socket<P> + AsyncReadOp + Cancel,
        F: Complete<(P::Socket, P::Endpoint), io::Error>,
    {
        fn call(self, this: &mut ThreadIoContext) {
            let soc = unsafe { &*self.soc };
            soc.add_read_op(this, Box::new(self), SystemError::default())
        }

        fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
            let soc = unsafe { &*self.soc };
            soc.add_read_op(this, self, SystemError::default())
        }
    }

    pub fn async_accept<P, S, F>(soc: &S, handler: F) -> F::Output
    where
        P: Protocol,
        S: Socket<P> + AsyncReadOp + Cancel,
        F: Handler<(P::Socket, P::Endpoint), io::Error>,
    {
        let (tx, rx) = handler.channel();
        if !soc.as_ctx().stopped() {
            soc.as_ctx().do_dispatch(AsyncAccept {
                soc: soc,
                handler: tx,
                _marker: PhantomData,
            });
        } else {
            soc.as_ctx().do_dispatch(
                Failure::new(OPERATION_CANCELED, tx),
            );
        }
        rx.yield_wait_for(soc, soc.as_timeout(TimeoutLoc::READ))
    }

    pub fn blocking_accept<P, S>(soc: &S, timeout: &Timeout) -> io::Result<(P::Socket, P::Endpoint)>
    where
        P: Protocol,
        S: Socket<P> + AsIoContext,
    {
        if soc.as_ctx().stopped() {
            return Err(OPERATION_CANCELED.into());
        }
        loop {
            match accept(soc) {
                Ok((acc, ep)) => {
                    let pro = soc.protocol().clone();
                    let acc = unsafe { P::Socket::from_raw_fd(soc.as_ctx(), acc, pro) };
                    return Ok((acc, ep));
                }
                Err(TRY_AGAIN) | Err(WOULD_BLOCK) => {
                    if let Err(err) = readable(soc, &timeout) {
                        return Err(err.into());
                    }
                }
                Err(INTERRUPTED) if !soc.as_ctx().stopped() => {}
                Err(err) => return Err(err.into()),
            }
        }
    }

    pub fn nonblocking_accept<P, S>(soc: &S) -> io::Result<(P::Socket, P::Endpoint)>
    where
        P: Protocol,
        S: Socket<P> + AsIoContext,
    {
        if soc.as_ctx().stopped() {
            return Err(OPERATION_CANCELED.into());
        }
        Ok(accept(soc).map(|(acc, ep)| {
            let pro = soc.protocol().clone();
            let acc = unsafe { P::Socket::from_raw_fd(soc.as_ctx(), acc, pro) };
            (acc, ep)
        })?)
    }
}
