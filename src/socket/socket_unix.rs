use crate::IoContext;
use crate::buffer::MsgBuf;
use crate::core::{Deadline, Event, Fd, Timeout};
use crate::error::{OsError, Result};
use crate::sockaddr::{SockAddr, SockLen};
use crate::socket_base::{Endpoint, EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::num::NonZero;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{mem, ptr};

/// Low-level UNIX-based socket type.
pub struct Socket {
    ctx: IoContext,
    fd: Fd,
}

#[cfg(doc)]
impl Drop for Socket {
    fn drop(&mut self) {}
}

impl Socket {
    pub(crate) unsafe fn from_raw_fd(ctx: IoContext, fd: Fd) -> Self {
        Socket { ctx: ctx, fd: fd }
    }

    pub fn new<P>(ctx: IoContext, pro: P) -> Result<Self>
    where
        P: Protocol,
    {
        let socktype: i32 = pro.socket_type().into();
        #[cfg(target_os = "linux")]
        let socktype = socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK;
        unsafe {
            match libc::socket(
                pro.family_type().into(),
                socktype,
                pro.protocol_type().into(),
            ) {
                -1 => Err(OsError::last()),
                soc => {
                    let soc = Fd::from_raw_fd(soc);
                    #[cfg(target_os = "macos")]
                    soc.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    soc.set_nonblock()?;
                    Ok(Socket { ctx: ctx, fd: soc })
                }
            }
        }
    }

    pub const fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn socketpair<P>(ctx: &IoContext, pro: P) -> Result<(Socket, Socket)>
    where
        P: Protocol,
    {
        let mut sv: [MaybeUninit<libc::c_int>; 2] = [const { MaybeUninit::uninit() }; 2];
        let socktype: i32 = pro.socket_type().into();
        #[cfg(target_os = "linux")]
        let socktype = socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK;
        unsafe {
            match libc::socketpair(
                pro.family_type().into(),
                socktype,
                pro.protocol_type().into(),
                sv[0].as_mut_ptr(),
            ) {
                -1 => Err(OsError::last()),
                _ => {
                    let sv = mem::transmute::<_, [libc::c_int; 2]>(sv);
                    let s1 = Fd::from_raw_fd(sv[0]);
                    let s2 = Fd::from_raw_fd(sv[1]);
                    #[cfg(target_os = "macos")]
                    s1.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    s2.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    s1.set_nonblock()?;
                    #[cfg(target_os = "macos")]
                    s2.set_nonblock()?;
                    Ok((
                        Socket {
                            ctx: ctx.clone(),
                            fd: s1,
                        },
                        Socket {
                            ctx: ctx.clone(),
                            fd: s2,
                        },
                    ))
                }
            }
        }
    }

    pub(crate) fn as_fd(&self) -> &Fd {
        &self.fd
    }

    pub fn bind<E>(&self, ep: &EndpointRef<E>) -> Result<()>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match libc::bind(self.fd.as_raw_fd(), sa, ep.sockaddr_len()) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn listen(&self, backlog: i32) -> Result<()> {
        unsafe {
            match libc::listen(self.fd.as_raw_fd(), backlog) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn connect<E>(&self, ep: &EndpointRef<E>) -> Result<()>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match libc::connect(self.fd.as_raw_fd(), sa, ep.sockaddr_len()) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn close(self) -> Result<()> {
        self.fd.close()
    }

    pub fn accept<E>(&self) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            #[cfg(target_os = "linux")]
            let res = libc::accept4(
                self.fd.as_raw_fd(),
                sa.as_mut_ptr().cast(),
                &mut sa_len,
                libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            );
            #[cfg(target_os = "macos")]
            let res = libc::accept(self.0.0, sa.as_mut_ptr().cast(), &mut sa_len);
            match res {
                -1 => Err(OsError::last()),
                soc => {
                    let soc = Fd::from_raw_fd(soc);
                    #[cfg(target_os = "macos")]
                    soc.set_cloexec()?;
                    #[cfg(target_os = "macos")]
                    soc.set_nonblock()?;
                    let ep = E::from_sockaddr(E::SockAddr::init(sa, sa_len));
                    Ok((
                        Socket {
                            ctx: self.ctx.clone(),
                            fd: soc,
                        },
                        ep,
                    ))
                }
            }
        }
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match libc::recv(self.fd.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn receive_from<E>(&self, buf: &mut [u8]) -> Result<(usize, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::recvfrom(
                self.fd.as_raw_fd(),
                buf.as_mut_ptr().cast(),
                buf.len(),
                0,
                sa.as_mut_ptr().cast(),
                &mut sa_len,
            ) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok((
                    len as usize,
                    E::from_sockaddr(E::SockAddr::init(sa, sa_len)),
                )),
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn receive_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        unsafe {
            match libc::recvmsg(self.0.0, mbuf.as_ptr(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn receive_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        if let Some(len) = mbuf.next() {
            Ok(len)
        } else {
            unsafe {
                mbuf.uninit();
                let mmsghdr = mbuf.as_mut_slice();
                match libc::recvmmsg(
                    self.fd.as_raw_fd(),
                    mmsghdr.as_mut_ptr(),
                    mmsghdr.len() as SockLen,
                    0,
                    ptr::null_mut(),
                ) {
                    -1 => Err(OsError::last()),
                    0 => Err(OsError::CONNECTION_ABORTED),
                    len => Ok(mbuf.set_len(len as usize)),
                }
            }
        }
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.receive_msg_impl(mbuf)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match libc::send(self.fd.as_raw_fd(), buf.as_ptr().cast(), buf.len(), 0) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn send_to<E>(&self, buf: &[u8], ep: &EndpointRef<E>) -> Result<usize>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match libc::sendto(
                self.fd.as_raw_fd(),
                buf.as_ptr().cast(),
                buf.len(),
                0,
                sa,
                ep.sockaddr_len() as SockLen,
            ) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn send_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        unsafe {
            match libc::sendmsg(self.0.0, mbuf.as_ptr(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }
    #[cfg(target_os = "linux")]
    fn send_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        if let Some(len) = mbuf.next() {
            Ok(len)
        } else {
            unsafe {
                let mmsghdr = mbuf.as_mut_slice();
                match libc::sendmmsg(self.fd.as_raw_fd(), mmsghdr.as_mut_ptr(), mmsghdr.len() as SockLen, 0) {
                    -1 => Err(OsError::last()),
                    0 => Err(OsError::CONNECTION_ABORTED),
                    len => Ok(mbuf.set_len(len as usize)),
                }
            }
        }
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.send_msg_impl(mbuf)
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        self.fd.read(buf)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        self.fd.write(buf)
    }

    pub fn getsockname<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::getsockname(self.fd.as_raw_fd(), sa.as_mut_ptr().cast(), &mut sa_len) {
                -1 => Err(OsError::last()),
                _ => Ok(E::from_sockaddr(E::SockAddr::init(sa, sa_len))),
            }
        }
    }

    pub fn getpeername<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::getpeername(self.fd.as_raw_fd(), sa.as_mut_ptr().cast(), &mut sa_len) {
                -1 => Err(OsError::last()),
                _ => Ok(E::from_sockaddr(E::SockAddr::init(sa, sa_len))),
            }
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        unsafe {
            match libc::shutdown(self.fd.as_raw_fd(), how as i32) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn setsockopt<P>(&self, pro: P, opt: &dyn SetSockOpt<P>) -> Result<()>
    where
        P: Protocol,
    {
        let (key, data) = opt.data(pro);
        unsafe {
            match libc::setsockopt(
                self.fd.as_raw_fd(),
                key.level,
                key.name,
                data.as_ptr().cast(),
                data.len() as SockLen,
            ) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn getsockopt<P, S>(&self, pro: P) -> Result<S>
    where
        P: Protocol,
        S: GetSockOpt<P>,
    {
        let (key, init) = S::init(pro);
        let mut data = MaybeUninit::<S>::uninit();
        let mut data_len = size_of::<S>() as SockLen;
        unsafe {
            match libc::getsockopt(
                self.fd.as_raw_fd(),
                key.level,
                key.name,
                data.as_mut_ptr().cast(),
                &mut data_len,
            ) {
                -1 => Err(OsError::last()),
                _ => Ok(init(data, data_len)),
            }
        }
    }

    pub fn poll_in(&self, timeout: Timeout) -> Result<()> {

        unsafe {
            let mut poll = libc::pollfd {
                fd: self.fd.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            match libc::poll(&mut poll, 1, timeout.0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::OPERATION_CANCELED),
                _ => Ok(()),
            }
        }
    }

    pub fn poll_out(&self, timeout: Timeout) -> Result<()> {
        unsafe {
            let mut poll = libc::pollfd {
            fd: self.fd.as_raw_fd(),
            events: libc::POLLOUT,
            revents: 0,
        };
            match libc::poll(&mut poll, 1, timeout.0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::OPERATION_CANCELED),
                _ => Ok(()),
            }
        }
    }
}

#[cfg(unix)]
pub(crate) struct WaitForReadable {
    ctx: IoContext,
    event: Event,
    timer: Deadline,
}

#[cfg(unix)]
impl Future for WaitForReadable {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self.event.lock().unwrap().read_poll(ctx) {
            Poll::Pending => {
                if self
                    .ctx
                    .inner
                    .scheduler
                    .insert_event(&self.event, self.timer)
                {
                    self.ctx.inner.reactor.intr.wake_up_alarm(self.timer)
                }
                Poll::Pending
            }
            Poll::Ready(res) => Poll::Ready(res),
        }
    }
}

pub(crate) struct WaitForWritable {
    ctx: IoContext,
    event: Event,
    timer: Deadline,
}

impl Future for WaitForWritable {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self.event.lock().unwrap().write_poll(ctx) {
            Poll::Pending => {
                if self
                    .ctx
                    .inner
                    .scheduler
                    .insert_event(&self.event, self.timer)
                {
                    let timer = self.timer;
                    self.ctx.inner.reactor.intr.wake_up_alarm(timer)
                }
                Poll::Pending
            }
            Poll::Ready(res) => Poll::Ready(res),
        }
    }
}


pub(crate) struct AsyncSocket {
    soc: crate::socket::Socket,
    event: Event,
}

impl Drop for AsyncSocket {
    fn drop(&mut self) {
        self.soc.as_ctx().inner.reactor.del_socket(self.soc.as_fd())
    }
}

impl AsyncSocket {
    pub(crate) fn new(soc: crate::socket::Socket) -> Self {
        let event: Event = Default::default();
        soc.as_ctx().inner.reactor.add_socket(soc.as_fd(), &event);
        Self {
            soc: soc,
            event: event,
        }
    }

    pub(crate) const fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }

    pub(crate) const fn as_socket(&self) -> &crate::socket::Socket {
        &self.soc
    }

    fn wake(&self) {
        if let Some(waker) = self.soc.as_ctx().inner.waker.lock().unwrap().take() {
            waker.wake();
        }
    }
}

impl AsyncSocket {
    fn poll_in(&self, timeout: Timeout) -> WaitForReadable {
        let timer = Deadline::new(timeout);
        self.wake();
        WaitForReadable {
            ctx: self.soc.ctx.clone(),
            event: self.event.clone(),
            timer: timer,
        }
    }

    fn poll_out(&self, timeout: Timeout) -> WaitForWritable {
        let timer = Deadline::new(timeout);
        self.wake();
        WaitForWritable {
            ctx: self.soc.ctx.clone(),
            event: self.event.clone(),
            timer: timer,
        }
    }

    pub(crate) async fn accept<E>(&self, timeout: Timeout) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        loop {
            match self.poll_in(timeout).await {
                Ok(()) => loop {
                    match self.as_socket().accept() {
                        Ok(soc) => return Ok(soc),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn connect<E>(&self, ep: &EndpointRef<'_, E>, timeout: Timeout) -> Result<()>
    where
        E: Endpoint,
    {
        loop {
            match self.as_socket().connect(ep) {
                Ok(_) => return Ok(()),
                Err(OsError::IN_PROGRESS) | Err(OsError::WOULD_BLOCK) => {
                    if let Err(err) = self.poll_out(timeout).await {
                        return Err(err);
                    }
                }
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn write_some(&self, buf: &[u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_out(timeout).await {
                Ok(()) => loop {
                    match self.as_socket().write(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn send(&self, buf: &[u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_out(timeout).await {
                Ok(()) => loop {
                    match self.as_socket().send(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn send_to<E>(
        &self,
        buf: &[u8],
        ep: &EndpointRef<'_, E>,
        timeout: Timeout,
    ) -> Result<usize>
    where
        E: Endpoint,
    {
        loop {
            match self.poll_out(timeout).await {
                Ok(()) => loop {
                    match self.as_socket().send_to(buf, ep) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn send_msg(&self, mbuf: &mut MsgBuf, timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_out(timeout).await {
                Ok(()) => loop {
                    match self.as_socket().send_msg(mbuf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn read_some(&self, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_in(timeout).await {
                Ok(()) => loop {
                    match self.as_socket().read(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn receive(&self, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_in(timeout).await {
                Ok(()) => loop {
                    match self.as_socket().receive(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn receive_from<E>(
        &self,
        buf: &mut [u8],
        timeout: Timeout,
    ) -> Result<(usize, E)>
    where
        E: Endpoint,
    {
        loop {
            match self.poll_in(timeout).await {
                Ok(()) => loop {
                    match self.as_socket().receive_from(buf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    pub(crate) async fn receive_msg(&self, mbuf: &mut MsgBuf, timeout: Timeout) -> Result<usize> {
        loop {
            match self.poll_in(timeout).await {
                Ok(()) => loop {
                    match self.as_socket().receive_msg(mbuf) {
                        Ok(len) => return Ok(len),
                        #[allow(unreachable_patterns)]
                        Err(OsError::TRY_AGAIN) | Err(OsError::WOULD_BLOCK) => break,
                        Err(OsError::INTERRUPTED) => {
                            if self.as_ctx().is_stopped() {
                                return Err(OsError::OPERATION_CANCELED);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                },
                Err(OsError::INTERRUPTED) => {
                    if self.as_ctx().is_stopped() {
                        return Err(OsError::OPERATION_CANCELED);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }
}
