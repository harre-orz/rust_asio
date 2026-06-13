use super::{AsyncSocket, Socket};
use crate::buffer::MsgBuf;
use crate::core::{IoContext};
use crate::error::{OsError, Result};
use crate::primitive::AsRawHandle;
use crate::primitive::{Deadline, Timeout};
use crate::sockaddr::{SockAddr, SockAddrWithLen, SockLen};
use crate::socket_base::{
    Endpoint, EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown, SockOpt,
};
use std::mem::MaybeUninit;
use std::{mem, ptr, slice};
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::System::IO;

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

    pub fn nb_read(&self, buf: &mut [u8]) -> Result<usize> {
        self.nb_receive(buf)
    }

    pub fn nb_recv(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match WinSock::recv(self.0, buf.as_mut_ptr().cast(), buf.len() as i32, 0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn nb_recvmsg(&self, mbuf: &mut MsgBuf, ctx: &IoContext) -> Result<usize> {
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

    pub fn nb_recvfrom<E>(&self, buf: &mut [u8]) -> Result<(usize, E)>
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

    pub fn nb_sendto<E>(&self, buf: &[u8], ep: &EndpointRef<E>) -> Result<usize>
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

    pub fn nb_sendmsg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
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

impl AsyncSocket {
    pub(crate) async fn async_accept<P>(&self, t: Timeout, pro: P) -> Result<(Socket, P::Endpoint)>
    where
        P: Protocol,
    {
        if self.as_ctx().is_stopped() {
            return Err(OsError::OPERATION_CANCELED);
        }
        let acc = Socket::new(pro)?;
        let mut addr_buf: [MaybeUninit<u8>; 1024] = [const { MaybeUninit::uninit() }; 1024];
        let addr_len = size_of::<P::Endpoint>() as u32 + 16;
        let mut _bytes = MaybeUninit::<u32>::uninit();
        loop {
            let mut _ov = MaybeUninit::<IO::OVERLAPPED>::zeroed();
            let event = self.event.lock();
            unsafe {
                if WinSock::AcceptEx(
                    self.as_socket().as_raw_socket(),
                    acc.0,
                    addr_buf[0].as_mut_ptr().cast(),
                    0,
                    addr_len,
                    addr_len,
                    _bytes.as_mut_ptr(),
                    _ov.as_mut_ptr(),
                ) == 0
                {
                    drop(event);
                    let err = OsError::last();
                    if err != OsError::IO_PENDING {
                        return Err(err);
                    }
                } else {
                    match event.poll_iocp(&self.event, t).await {
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
                }
            }
        }
    }

    pub(crate) async fn async_connect<E>(&self, ep: &EndpointRef<'_, E>, t: Timeout) -> Result<()>
    where
        E: Endpoint,
    {
        self.as_socket().bind(&ep.unspecified())?;
        loop {
            let mut _ov = MaybeUninit::zeroed();
            unsafe {
                let event = self.event.lock();
                if (self.as_ctx().winsock().ConnectEx)(
                    self.as_socket().as_raw_socket(),
                    ep.sockaddr_ref(),
                    ep.sockaddr_len(),
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    _ov.as_mut_ptr().cast(),
                ) == 0
                {
                    drop(event);
                    let err = OsError::last();
                    if err != OsError::IO_PENDING {
                        return Err(err);
                    }
                } else {
                    match event.poll_iocp(self.event, t).await {
                        Err(err) => return Err(err),
                        Ok(_) => return Ok(()),
                    }
                }
            }
        }
    }

    pub(crate) async fn async_write(&self, buf: &[u8], timeout: Timeout) -> Result<usize> {
        self.async_send(buf, timeout).await
    }

    pub(crate) async fn async_send(&self, buf: &[u8], t: Timeout) -> Result<usize> {
        let io = WinSock::WSABUF {
            len: buf.len() as u32,
            buf: buf.as_ptr().cast_mut(),
        };
        let mut flags = 0;
        let mut _bytes = MaybeUninit::<u32>::uninit();
        loop {
            let mut _ov = MaybeUninit::<IO::OVERLAPPED>::zeroed();
            let event = self.event.lock();
            unsafe {
                match WinSock::WSASend(
                    self.as_socket().as_raw_socket(),
                    &io,
                    1,
                    _bytes.as_mut_ptr(),
                    flags,
                    _ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        drop(event);
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => return event.poll_iocp(&self.event, t).await,
                }
            }
        }
    }

    pub(crate) async fn async_sendto<E>(
        &self,
        buf: &[u8],
        ep: &EndpointRef<'_, E>,
        t: Timeout,
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
        loop {
            let mut _ov = MaybeUninit::<IO::OVERLAPPED>::zeroed();
            let event = self.event.lock();
            unsafe {
                match WinSock::WSASendTo(
                    self.as_socket().as_raw_socket(),
                    &io,
                    1,
                    _bytes.as_mut_ptr(),
                    flags,
                    ep.sockaddr_ref(),
                    ep.sockaddr_len(),
                    _ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        drop(event);
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => return event.poll_iocp(&self.event, t).await,
                }
            }
        }
    }

    pub(crate) async fn async_sendmsg(&self, mbuf: &mut MsgBuf, t: Timeout) -> Result<usize> {
        let mut _bytes = MaybeUninit::<u32>::uninit();
        loop {
            let mut _ov = MaybeUninit::<IO::OVERLAPPED>::zeroed();
            let event = self.event.lock();
            unsafe {
                match (self.as_ctx().winsock().WSARecvMsg)(
                    self.as_socket().as_raw_socket(),
                    mbuf.as_ptr(),
                    _bytes.as_mut_ptr(),
                    _ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        drop(event);
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => return event.poll_iocp(&self.event, t).await,
                }
            }
        }
    }

    pub(crate) async fn async_read(&self, buf: &mut [u8], timeout: Timeout) -> Result<usize> {
        self.async_recv(buf, timeout).await
    }

    pub(crate) async fn async_recv(&self, buf: &mut [u8], t: Timeout) -> Result<usize> {
        let io = WinSock::WSABUF {
            len: buf.len() as u32,
            buf: buf.as_mut_ptr(),
        };
        let mut flags = 0;
        let mut _bytes = MaybeUninit::<u32>::uninit();
        loop {
            let mut _ov = MaybeUninit::<IO::OVERLAPPED>::zeroed();
            let event = self.event.lock();
            unsafe {
                match WinSock::WSARecv(
                    self.as_socket().as_raw_socket(),
                    &io,
                    1,
                    _bytes.as_mut_ptr(),
                    &mut flags,
                    _ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        drop(event);
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => return event.poll_iocp(&self.event, t).await,
                }
            }
        }
    }

    pub(crate) async fn async_recvfrom<E>(&self, buf: &mut [u8], t: Timeout) -> Result<(usize, E)>
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
        loop {
            let mut _ov = MaybeUninit::<IO::OVERLAPPED>::zeroed();
            let event = self.event.lock();
            unsafe {
                match WinSock::WSARecvFrom(
                    self.as_socket().as_raw_socket(),
                    &io,
                    1,
                    _bytes.as_mut_ptr(),
                    &mut flags,
                    sa.as_mut_ptr(),
                    &mut sa_len,
                    _ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        drop(event);
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => match event.poll_iocp(&self.event, t).await {
                        Err(err) => return Err(err),
                        Ok(len) => return Ok((len, sa.assume_init())),
                    },
                }
            }
        }
    }

    pub(crate) async fn async_recvmsg(&self, mbuf: &mut MsgBuf, t: Timeout) -> Result<usize> {
        let mut _bytes = MaybeUninit::<u32>::uninit();
        loop {
            let mut _ov = MaybeUninit::<IO::OVERLAPPED>::zeroed();
            let event = self.event.lock();
            unsafe {
                match (self.as_ctx().winsock().WSARecvMsg)(
                    self.as_socket().as_raw_socket(),
                    mbuf.as_ptr(),
                    _bytes.as_mut_ptr(),
                    _ov.as_mut_ptr(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => {
                        drop(event);
                        let err = OsError::last();
                        if err != OsError::IO_PENDING {
                            return Err(err);
                        }
                    }
                    _ => return event.poll_iocp(&self.event, t).await,
                }
            }
        }
    }
}
