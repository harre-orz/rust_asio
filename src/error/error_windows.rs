use std::ffi::{CStr, OsStr, OsString};
use std::mem::MaybeUninit;
use std::num::NonZero;
use std::os::windows::ffi::OsStringExt;
use std::{fmt, mem, ptr};
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::System::Diagnostics::Debug;

/// The OS specified error code.
///
/// https://learn.microsoft.com/en-us/windows/win32/winsock/windows-sockets-error-codes-2
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OsError(NonZero<WinSock::WSA_ERROR>);

impl OsError {
    fn new(value: WinSock::WSA_ERROR) -> Self {
        Self(NonZero::new(value).unwrap())
    }

    /// Specified event object handle is invalid.
    pub const INVALID_HANDLE: Self = Self::new(WinSock::WSA_INVALID_HANDLE);

    /// Insufficient memory available.
    pub const NOT_ENOUGH_MEMORY: Self = Self::new(WinSock::WSA_NOT_ENOUGH_MEMORY);

    /// One or more parameters are invalid.
    pub const INVALID_PARAMETER: Self = Self::new(WinSock::WSA_INVALID_PARAMETER);

    /// Overlapped operation aborted.
    pub const OPERATION_ABORTED: Self = Self::new(WinSock::WSA_OPERATION_ABORTED);

    /// Overlapped I/O event object not in signaled state.
    pub const IO_INCOMPLETE: Self = Self::new(WinSock::WSA_IO_INCOMPLETE);

    /// Overlapped operations will complete later.
    pub const IO_PENDING: Self = Self::new(WinSock::WSA_IO_PENDING);

    /// Interrupted function call.
    pub const INTERRUPTED: Self = Self::new(WinSock::WSAEINTR);

    /// Bad file descriptor.
    pub const BAD_DESCRIPTOR: Self = Self::new(WinSock::WSAEBADF);

    /// Permission denied.
    pub const ACCESS_DENIED: Self = Self::new(WinSock::WSAEACCES);

    /// Bad address.
    pub const FAULT: Self = Self::new(WinSock::WSAEFAULT);

    /// Invalid argument.
    pub const INVALID_ARGUMENT: Self = Self::new(WinSock::WSAEINVAL);

    /// Too many open files.
    pub const NO_DESCRIPTORS: Self = Self::new(WinSock::WSAEMFILE);

    /// Resource temporarily unavailable.
    pub const WOULD_BLOCK: Self = Self::new(WinSock::WSAEWOULDBLOCK);

    /// peration now in progress.
    pub const IN_PROGRESS: Self = Self::new(WinSock::WSAEINPROGRESS);

    /// Operation already in progress.
    pub const ALREADY_STARTED: Self = Self::new(WinSock::WSAEALREADY);

    /// Socket operation on non-socket.
    pub const NOT_SOCKET: Self = Self::new(WinSock::WSAENOTSOCK);

    // WSAEDESTADDRREQ

    /// Message to long.
    pub const MESSAGE_SIZE: Self = Self::new(WinSock::WSAEMSGSIZE);

    // WSAEPROTOTYPE

    /// Protocol not available.
    pub const NO_PROTOCOL_OPTION: Self = Self::new(WinSock::WSAENOPROTOOPT);

    // WSAEPROTONOSUPPORT

    // WSAESOCKTNOSUPPORT

    /// Operation not supported.
    pub const OPERATION_NOT_SUPPORTED: Self = Self::new(WinSock::WSAEOPNOTSUPP);

    // WSAEPFNOSUPPORT

    /// Address family not supported by protocol.
    pub const ADDRESS_FAMILY_NOT_SUPPORTED: Self = Self::new(WinSock::WSAEAFNOSUPPORT);

    /// Address already in use.
    pub const ADDRESS_IN_USE: Self = Self::new(WinSock::WSAEADDRINUSE);

    // WSAEADDRNOTAVAIL

    /// Network is down.
    pub const NETWORK_DOWN: Self = Self::new(WinSock::WSAENETDOWN);

    /// Network is unreachable.
    pub const NETWORK_UNREACHABLE: Self = Self::new(WinSock::WSAENETUNREACH);

    /// Network dropped connection on reset.
    pub const NETWORK_RESET: Self = Self::new(WinSock::WSAENETRESET);

    /// A connection has been aborted.
    pub const CONNECTION_ABORTED: Self = Self::new(WinSock::WSAECONNABORTED);

    /// Connection reset by peer.
    pub const CONNECTION_RESET: Self = Self::new(WinSock::WSAECONNRESET);

    /// No buffer space available.
    pub const NO_BUFFER_SPACE: Self = Self::new(WinSock::WSAENOBUFS);

    /// Transport endpoint is already connected.
    pub const ALREADY_CONNECTED: Self = Self::new(WinSock::WSAEISCONN);

    /// Transport endpoint is not connected.
    pub const NOT_CONNECTED: Self = Self::new(WinSock::WSAENOTCONN);

    /// Cannot send after transport endpoint shutdown.
    pub const SHUT_DOWN: Self = Self::new(WinSock::WSAESHUTDOWN);

    // WSAETOOMANYREFS

    /// Connection timed out.
    pub const TIMED_OUT: Self = Self::new(WinSock::WSAETIMEDOUT);

    /// connection refused.
    pub const CONNECTION_REFUSED: Self = Self::new(WinSock::WSAECONNREFUSED);

    // WSAELOOP

    /// The name was too long.
    pub const NAME_TOO_LONG: Self = Self::new(WinSock::WSAENAMETOOLONG);

    // WSAEHOSTDOWN

    /// No route to host.
    pub const HOST_UNREACHABLE: Self = Self::new(WinSock::WSAEHOSTUNREACH);

    pub(crate) unsafe fn last() -> Self {
        let errno = WinSock::WSAGetLastError();
        mem::transmute(errno)
    }

    pub(super) fn code(&self) -> WinSock::WSA_ERROR {
        self.0.get()
    }
}

impl fmt::Display for OsError {
    /// Returns a string describing error code.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        const BUF_LEN: usize = 1024;

        let mut buf: [MaybeUninit<_>; BUF_LEN] = [const { MaybeUninit::uninit() }; BUF_LEN];
        unsafe {
            // https://learn.microsoft.com/ja-jp/windows/win32/api/winbase/nf-winbase-formatmessagew
            match Debug::FormatMessageW(
                Debug::FORMAT_MESSAGE_FROM_SYSTEM | Debug::FORMAT_MESSAGE_IGNORE_INSERTS,
                ptr::null(),
                self.code() as u32,
                0,
                buf[0].as_mut_ptr(),
                buf.len() as u32,
                ptr::null(),
            ) {
                0 => {
                    write!(f, "Unknown error {}", self.0.get())
                }
                len => {
                    let buf = mem::transmute::<_, [u16; BUF_LEN]>(buf);
                    let buf = buf[..len as usize];
                    write!(f, "{}", OsString::from_wide(&buf).to_string_lossy())
                }
            }
        }
    }
}
