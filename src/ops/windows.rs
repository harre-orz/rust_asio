use crate::buffer::MsgBuf;
use crate::error::{OsError, Result};
use crate::exec::AsyncSocket;
use crate::sockaddr::{SockAddrWithLen, SockLen};
use crate::socket::{Socket, Timeout};
use crate::socket_base::{Endpoint, EndpointRef, Protocol, SetSockOpt, SockOpt};
use chrono::Month::May;
use std::mem::MaybeUninit;
use std::{mem, ptr, slice};
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::System::IO;

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

pub(crate) async fn async_accept<P>(
    soc: &AsyncSocket,
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
                soc.as_socket().as_raw_socket(),
                acc.as_raw_socket(),
                addr_buf[0].as_mut_ptr().cast(),
                0,
                addr_len,
                addr_len,
                _bytes.as_mut_ptr(),
                ov.as_mut_ptr(),
            ) > 0
            {
                match soc.iocp(timeout, ov.assume_init()).await {
                    Ok(_) => {
                        acc.setsockopt(pro, &UpdateAccept(soc.as_socket().as_raw_socket()))?;
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
                        let ep =
                            P::Endpoint::from_sockaddr(SockAddrWithLen::new_unchecked(sa, sa_len));
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
    soc: &AsyncSocket,
    ep: &EndpointRef<'_, E>,
    timeout: Timeout,
) -> Result<()>
where
    E: Endpoint,
{
    soc.as_socket().bind(&ep.unspecified())?;
    loop {
        unsafe {
            let mut ov: MaybeUninit<IO::OVERLAPPED> = MaybeUninit::zeroed();
            if (soc.as_ctx().winsock().ConnectEx)(
                soc.as_socket().as_raw_socket(),
                ep.sockaddr_ref(),
                ep.sockaddr_len(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                ov.as_mut_ptr().cast(),
            ) > 0
            {
                match soc.iocp(timeout, ov.assume_init()).await {
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

pub(crate) async fn async_write_some(
    soc: &AsyncSocket,
    buf: &[u8],
    timeout: Timeout,
) -> Result<usize> {
    async_send(soc, buf, timeout).await
}

pub(crate) async fn async_send(soc: &AsyncSocket, buf: &[u8], timeout: Timeout) -> Result<usize> {
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
                soc.as_socket().as_raw_socket(),
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
                _ => return soc.iocp(timeout, ov.assume_init()).await,
            }
        }
    }
}

pub(crate) async fn async_send_to<E>(
    soc: &AsyncSocket,
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
                soc.as_socket().as_raw_socket(),
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
                _ => return soc.iocp(timeout, ov.assume_init()).await,
            }
        }
    }
}

pub(crate) async fn async_send_msg(
    soc: &AsyncSocket,
    mbuf: &mut MsgBuf,
    timeout: Timeout,
) -> Result<usize> {
    let mut _bytes = MaybeUninit::<u32>::uninit();
    let mut ov = MaybeUninit::<IO::OVERLAPPED>::uninit();
    loop {
        unsafe {
            match (soc.as_ctx().winsock().WSARecvMsg)(
                soc.as_socket().as_raw_socket(),
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
                _ => return soc.iocp(timeout, ov.assume_init()).await,
            }
        }
    }
}

pub(crate) async fn async_read_some(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<usize> {
    async_receive(soc, buf, timeout).await
}

pub(crate) async fn async_receive(
    soc: &AsyncSocket,
    buf: &mut [u8],
    timeout: Timeout,
) -> Result<usize> {
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
                soc.as_socket().as_raw_socket(),
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
                _ => return soc.iocp(timeout, ov.assume_init()).await,
            }
        }
    }
}

pub(crate) async fn async_receive_from<E>(
    soc: &AsyncSocket,
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
                soc.as_socket().as_raw_socket(),
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
                _ => match soc.iocp(timeout, ov.assume_init()).await {
                    Err(err) => return Err(err),
                    Ok(len) => return Ok((len, sa.assume_init())),
                },
            }
        }
    }
}

pub(crate) async fn async_receive_msg(
    soc: &AsyncSocket,
    mbuf: &mut MsgBuf,
    timeout: Timeout,
) -> Result<usize> {
    let mut _bytes = MaybeUninit::<u32>::uninit();
    let mut ov = MaybeUninit::<IO::OVERLAPPED>::uninit();
    loop {
        unsafe {
            match (soc.as_ctx().winsock().WSARecvMsg)(
                soc.as_socket().as_raw_socket(),
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
                _ => return soc.iocp(timeout, ov.assume_init()).await,
            }
        }
    }
}
