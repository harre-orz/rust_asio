use crate::error::{OsError, Result};
use crate::exec::AsyncSocket;
use crate::socket::{Socket, Timeout};
use std::mem::MaybeUninit;
use std::ptr;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::System::IO;
use crate::buffer::MsgBuf;
use crate::socket_base::{Endpoint, EndpointRef, Protocol};

pub(crate) async fn async_accept<P>(soc: &AsyncSocket, pro: P, timeout: Timeout) -> Result<(AsyncSocket, P::Endpoint)>
where
    P: Protocol,
{
    unimplemented!()
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
                ep.sockaddr_ref(), ep.sockaddr_len(),
                ptr::null_mut(), 0, ptr::null_mut(),
                ov.as_mut_ptr().cast()
            ) > 0 {
                return soc.iocp(timeout, ov.assume_init()).await.map(|_| ())
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
    let buf = WinSock::WSABUF {
        len: 1,
        buf: buf.as_ptr() as *mut _,
    };
    loop {
        unsafe {
            match WinSock::WSASend(
                soc.as_socket().as_raw_socket(),
                &buf, 1,
                bytes, flags,
                ov, None,
            ) {
                SOCKET_
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
    unimplemented!()
}

pub(crate) async fn async_send_msg(
    soc: &AsyncSocket,
    mbuf: &mut MsgBuf,
    timeout: Timeout,
) -> Result<usize> {
    unimplemented!()
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
    unimplemented!()
}

pub(crate) async fn async_receive_msg(
    soc: &AsyncSocket,
    mbuf: &mut MsgBuf,
    timeout: Timeout,
) -> Result<usize> {
    unimplemented!()
}