use crate::error::{OsError, Result};
use crate::exec::AsyncSocket;
use crate::socket::Timeout;
use std::mem::MaybeUninit;
use std::ptr;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::System::IO;

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
                _ => return soc.iocp(timeout).await,
            }
        }
    }
}
