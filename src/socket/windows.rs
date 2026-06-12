use crate::buffer::MsgBuf;
use crate::error::{OsError, Result};
use crate::core::{Event, IoContext};
use crate::sockaddr::{SockAddr, SockAddrWithLen, SockLen};
use crate::socket_base::{
    Endpoint, EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown, SockOpt,
};
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{mem, ptr, slice};
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::System::IO;
use super::Socket;
use crate::primitive::{Timeout, Deadline};
use crate::primitive::AsRawHandle;

impl Socket {
    pub fn new<P>(pro: P) -> Result<Self>
    where
        P: Protocol,
    {
        unsafe {
            match WinSock::WSASocketW(
                pro.family_type().into(),
                pro.socket_type().into(),
                pro.protocol_type().into(),
                ptr::null_mut(),
                0,
                WinSock::WSA_FLAG_OVERLAPPED,
            ) {
                SOCKET_ERROR_ => Err(OsError::last()),
                soc => {
                    let soc = Socket(soc);
                    soc.set_nonblock()?;
                    Ok(soc)
                }
            }
        }
    }

    fn set_nonblock(&self) -> Result<()> {
        let mut val = 0;
        unsafe {
            match WinSock::ioctlsocket(self.0, WinSock::FIONBIO, &mut val) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn close(self) -> Result<()> {
        unsafe {
            match WinSock::closesocket(self.0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn bind<E>(&self, ep: &EndpointRef<E>) -> Result<()>
    where
        E: Endpoint,
    {
        let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
        unsafe {
            match WinSock::bind(self.0, sa, ep.sockaddr_len()) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn listen(&self, backlog: i32) -> Result<()> {
        unsafe {
            match WinSock::listen(self.0, backlog) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn nb_connect<E>(&self, ep: &EndpointRef<E>) -> Result<()>
    where
        E: Endpoint,
    {
        let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
        unsafe {
            match WinSock::connect(self.0, sa, ep.sockaddr_len()) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn nb_accept<E>(&self) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match WinSock::accept(self.0, sa.as_mut_ptr().cast(), &mut sa_len) {
                SOCKET_ERROR_ => Err(OsError::last()),
                soc => {
                    let soc = Socket(soc);
                    let ep = E::from_sockaddr(E::SockAddr::init(sa, sa_len));
                    Ok((soc, ep))
                }
            }
        }
    }

    pub fn nb_read_some(&self, buf: &mut [u8]) -> Result<usize> {
        self.nb_receive(buf)
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match WinSock::recv(self.0, buf.as_mut_ptr().cast(), buf.len() as i32, 0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn nb_receive_msg(&self, mbuf: &mut MsgBuf, ctx: &IoContext) -> Result<usize> {
        let mut len = MaybeUninit::<u32>::uninit();
        unsafe {
            match (ctx.winsock().WSARecvMsg)(
                self.0,
                mbuf.as_ptr(),
                len.as_mut_ptr(),
                ptr::null_mut(),
                None,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => {
                    let len = len.assume_init();
                    if len == 0 {
                        Err(OsError::CONNECTION_ABORTED)
                    } else {
                        Ok(len as usize)
                    }
                }
            }
        }
    }

    pub fn nb_receive_from<E>(&self, buf: &mut [u8]) -> Result<(usize, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match WinSock::recvfrom(
                self.0,
                buf.as_mut_ptr().cast(),
                buf.len() as i32,
                0,
                sa.as_mut_ptr().cast(),
                &mut sa_len,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => {
                    let sa = E::SockAddr::init(sa, sa_len);
                    Ok((len as usize, E::from_sockaddr(sa)))
                }
            }
        }
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match WinSock::send(self.0, buf.as_ptr().cast(), buf.len() as i32, 0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn nb_send_to<E>(&self, buf: &[u8], ep: &EndpointRef<E>) -> Result<usize>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match WinSock::sendto(
                self.0,
                buf.as_ptr().cast(),
                buf.len() as i32,
                0,
                sa,
                ep.sockaddr_len(),
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn nb_send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        unsafe {
            match WinSock::WSASendMsg(
                self.0,
                mbuf.as_ptr(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                None,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn nb_write(&self, buf: &[u8]) -> Result<usize> {
        self.nb_send(buf)
    }

    pub fn getsockname<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match WinSock::getsockname(self.0, sa.as_mut_ptr().cast(), &mut sa_len) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => {
                    let ep = E::SockAddr::init(sa, sa_len);
                    Ok(E::from_sockaddr(ep))
                }
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
            match WinSock::getpeername(self.0, sa.as_mut_ptr().cast(), &mut sa_len) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => {
                    let ep = E::SockAddr::init(sa, sa_len);
                    Ok(E::from_sockaddr(ep))
                }
            }
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        unsafe {
            match WinSock::shutdown(self.0, how as WinSock::WINSOCK_SHUTDOWN_HOW) {
                WinSock::SOCKET_ERROR => Err(unsafe { OsError::last() }),
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
            match WinSock::getsockopt(
                self.0,
                key.level,
                key.name,
                data.as_mut_ptr().cast(),
                &mut data_len,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(init(data, data_len)),
            }
        }
    }

    pub fn setsockopt<P>(&self, pro: P, opt: &dyn SetSockOpt<P>) -> Result<()>
    where
        P: Protocol,
    {
        let (key, data) = opt.data(pro);
        unsafe {
            match WinSock::setsockopt(
                self.0,
                key.level,
                key.name,
                data.as_ptr().cast(),
                data.len() as SockLen,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
    pub fn poll_in(&self, timeout: Timeout) -> Result<()> {
        let mut poll = WinSock::WSAPOLLFD {
            fd: self.0,
            events: WinSock::POLLIN,
            revents: 0,
        };
        unsafe {
            match WinSock::WSAPoll(&mut poll, 1, timeout.millis()) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn poll_out(&self, timeout: Timeout) -> Result<()> {
        let mut poll = WinSock::WSAPOLLFD {
            fd: self.0,
            events: WinSock::POLLOUT,
            revents: 0,
        };
        unsafe {
            match WinSock::WSAPoll(&mut poll, 1, timeout.millis()) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
}

impl AsRawHandle for Socket {
    unsafe fn as_raw_handle(&self) -> Foundation::HANDLE {
        self.0 as Foundation::HANDLE
    }
}

pub struct UpdateAccept(WinSock::SOCKET);

impl UpdateAccept {
    const KEY: SockOpt = SockOpt {
        level: WinSock::SOL_SOCKET,
        name: WinSock::SO_UPDATE_ACCEPT_CONTEXT,
    };

    const fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(ptr::from_ref(&self.0).cast(), size_of_val(&self.0)) }
    }
}

impl<P> SetSockOpt<P> for UpdateAccept
where
    P: Protocol,
{
    fn data(&self, _: P) -> (SockOpt, &[u8]) {
        (Self::KEY, self.as_bytes())
    }
}

pub struct WaitForIocp {
    ctx: IoContext,
    event: Event,
    timer: Deadline,
    ov: IO::OVERLAPPED,
}

impl Future for WaitForIocp {
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        self.event.poll(ctx)
    }
}

pub(crate) struct AsyncSocket {
    ctx: IoContext,
    soc: Socket,
    event: Event,
}

impl Drop for AsyncSocket {
    fn drop(&mut self) {
        self.ctx.inner.reactor.del_socket(&self.soc)
    }
}

impl AsyncSocket {
    pub(crate) fn new(ctx: IoContext, soc: Socket) -> Self {
        let event: Event = Default::default();
        ctx.inner.reactor.add_socket(&soc, &event);
        Self {
            ctx: ctx,
            soc: soc,
            event: event,
        }
    }

    pub(crate) const fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub(crate) const fn as_socket(&self) -> &Socket {
        &self.soc
    }

    fn wake(&self) {
        if let Some(waker) = self.ctx.inner.waker.lock().unwrap().take() {
            waker.wake();
        }
    }

    pub(crate) async fn async_accept<P>(
        &self,
        timeout: Timeout,
        pro: P,
    ) -> Result<(Socket, P::Endpoint)>
    where
        P: Protocol,
    {
        let acc = Socket::new(pro)?;
        let mut addr_buf: [MaybeUninit<u8>; 1024] = [const { MaybeUninit::uninit() }; 1024];
        let addr_len = size_of::<P::Endpoint>() as u32 + 16;
        let mut _bytes = MaybeUninit::<u32>::uninit();
        let mut ov = MaybeUninit::<IO::OVERLAPPED>::uninit();
        loop {
            unsafe {
                if WinSock::AcceptEx(
                    self.as_socket().as_raw_socket(),
                    acc.0,
                    addr_buf[0].as_mut_ptr().cast(),
                    0,
                    addr_len,
                    addr_len,
                    _bytes.as_mut_ptr(),
                    ov.as_mut_ptr(),
                ) > 0
                {
                    match self.iocp(timeout, ov.assume_init()).await {
                        Ok(_) => {
                            acc.setsockopt(pro, &UpdateAccept(self.as_socket().as_raw_socket()))?;
                            let addr_buf = mem::transmute::<_, [u8; 1024]>(addr_buf);
                            let mut _local_addr = MaybeUninit::<*mut WinSock::SOCKADDR>::uninit();
                            let mut _local_len = MaybeUninit::<i32>::uninit();
                            let mut remote_addr = MaybeUninit::<*mut WinSock::SOCKADDR>::uninit();
                            let mut remote_len = MaybeUninit::<i32>::uninit();
                            WinSock::GetAcceptExSockaddrs(
                                addr_buf.as_ptr().cast(),
                                0,
                                addr_len,
                                addr_len,
                                _local_addr.as_mut_ptr(),
                                _local_len.as_mut_ptr(),
                                remote_addr.as_mut_ptr(),
                                remote_len.as_mut_ptr(),
                            );
                            let sa = remote_addr.assume_init();
                            let sa_len = remote_len.assume_init();
                            let ep = P::Endpoint::from_sockaddr(SockAddrWithLen::new_unchecked(
                                sa, sa_len,
                            ));
                            return Ok((acc, ep));
                        }
                        Err(err) => return Err(err),
                    }
                } else {
                    let err = OsError::last();
                    if err != OsError::IO_PENDING {
                        return Err(err);
                    }
                }
            }
        }
    }

    pub(crate) async fn async_connect<E>(
        &self,
        ep: &EndpointRef<'_, E>,
        timeout: Timeout,
    ) -> Result<()>
    where
        E: Endpoint,
    {
        self.as_socket().bind(&ep.unspecified())?;
        loop {
            unsafe {
                let mut ov: MaybeUninit<IO::OVERLAPPED> = MaybeUninit::zeroed();
                if (self.as_ctx().winsock().ConnectEx)(
                    self.as_socket().as_raw_socket(),
                    ep.sockaddr_ref(),
                    ep.sockaddr_len(),
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    ov.as_mut_ptr().cast(),
                ) > 0
                {
                    match self.iocp(timeout, ov.assume_init()).await {
                        Err(err) => return Err(err),
                        Ok(_) => return Ok(()),
                    }
                } else {
                    let err = OsError::last();
                    if err != OsError::IO_PENDING {
                        return Err(err);
                    }
                }
            }
        }
    }

    pub(crate) async fn async_write_some(&self, buf: &[u8], timeout: Timeout) -> Result<usize> {
        self.async_send(buf, timeout).await
    }

    pub(crate) async fn async_send(&self, buf: &[u8], timeout: Timeout) -> Result<usize> {
        let io = WinSock::WSABUF {
            len: buf.len() as u32,
            buf: buf.as_ptr().cast_mut(),
        };
        let mut flags = 0;
        let mut _bytes = MaybeUninit::<u32>::uninit();
        let mut ov = MaybeUninit::<IO::OVERLAPPED>::uninit();
        loop {
            unsafe {
                match WinSock::WSASend(
                    self.as_socket().as_raw_socket(),
                    &io,
                    1,
                    _bytes.as_mut_ptr(),
                    flags,
                    ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => return self.iocp(timeout, ov.assume_init()).await,
                }
            }
        }
    }

    pub(crate) async fn async_send_to<E>(
        &self,
        buf: &[u8],
        ep: &EndpointRef<'_, E>,
        timeout: Timeout,
    ) -> Result<usize>
    where
        E: Endpoint,
    {
        let io = WinSock::WSABUF {
            len: buf.len() as u32,
            buf: buf.as_ptr().cast_mut(),
        };
        let flags = 0;
        let mut _bytes = MaybeUninit::<u32>::uninit();
        let mut ov = MaybeUninit::<IO::OVERLAPPED>::uninit();
        loop {
            unsafe {
                match WinSock::WSASendTo(
                    self.as_socket().as_raw_socket(),
                    &io,
                    1,
                    _bytes.as_mut_ptr(),
                    flags,
                    ep.sockaddr_ref(),
                    ep.sockaddr_len(),
                    ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => return self.iocp(timeout, ov.assume_init()).await,
                }
            }
        }
    }

    pub(crate) async fn async_send_msg(
        &self,
        mbuf: &mut MsgBuf,
        timeout: Timeout,
    ) -> Result<usize> {
        let mut _bytes = MaybeUninit::<u32>::uninit();
        let mut ov = MaybeUninit::<IO::OVERLAPPED>::uninit();
        loop {
            unsafe {
                match (self.as_ctx().winsock().WSARecvMsg)(
                    self.as_socket().as_raw_socket(),
                    mbuf.as_ptr(),
                    _bytes.as_mut_ptr(),
                    ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => return self.iocp(timeout, ov.assume_init()).await,
                }
            }
        }
    }

    pub(crate) async fn async_read_some(&self, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
        self.async_receive(buf, timeout).await
    }

    pub(crate) async fn async_receive(&self, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
        let io = WinSock::WSABUF {
            len: buf.len() as u32,
            buf: buf.as_mut_ptr(),
        };
        let mut flags = 0;
        let mut _bytes = MaybeUninit::<u32>::uninit();
        let mut ov = MaybeUninit::<IO::OVERLAPPED>::uninit();
        loop {
            unsafe {
                match WinSock::WSARecv(
                    self.as_socket().as_raw_socket(),
                    &io,
                    1,
                    _bytes.as_mut_ptr(),
                    &mut flags,
                    ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => return self.iocp(timeout, ov.assume_init()).await,
                }
            }
        }
    }

    pub(crate) async fn async_receive_from<E>(
        &self,
        buf: &mut [u8],
        timeout: Timeout,
    ) -> Result<(usize, E)>
    where
        E: Endpoint,
    {
        let io = WinSock::WSABUF {
            len: buf.len() as u32,
            buf: buf.as_mut_ptr(),
        };
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        let mut flags = 0;
        let mut _bytes = MaybeUninit::<u32>::uninit();
        let mut ov = MaybeUninit::<IO::OVERLAPPED>::uninit();
        loop {
            unsafe {
                match WinSock::WSARecvFrom(
                    self.as_socket().as_raw_socket(),
                    &io,
                    1,
                    _bytes.as_mut_ptr(),
                    &mut flags,
                    sa.as_mut_ptr(),
                    &mut sa_len,
                    ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => match self.iocp(timeout, ov.assume_init()).await {
                        Err(err) => return Err(err),
                        Ok(len) => return Ok((len, sa.assume_init())),
                    },
                }
            }
        }
    }

    pub(crate) async fn async_receive_msg(
        &self,
        mbuf: &mut MsgBuf,
        timeout: Timeout,
    ) -> Result<usize> {
        let mut _bytes = MaybeUninit::<u32>::uninit();
        let mut ov = MaybeUninit::<IO::OVERLAPPED>::uninit();
        loop {
            unsafe {
                match (self.as_ctx().winsock().WSARecvMsg)(
                    self.as_socket().as_raw_socket(),
                    mbuf.as_ptr(),
                    _bytes.as_mut_ptr(),
                    ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => return self.iocp(timeout, ov.assume_init()).await,
                }
            }
        }
    }
}
