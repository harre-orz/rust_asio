//

#![allow(dead_code)]

use libc;
use socket_base::NativeHandle;
use std::ffi::CStr;
use std::fmt;
use std::io;
use std::mem;

/// The OS specified error code.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ErrorCode(i32);

impl From<ErrorCode> for io::Error {
    fn from(err: ErrorCode) -> Self {
        io::Error::from_raw_os_error(err.0)
    }
}

impl ErrorCode {
    /// Returns a last error.
    pub fn last_error() -> Self {
        ErrorCode(unsafe { *libc::__errno_location() })
    }

    /// Returns a socket error.
    pub fn socket_error(fd: NativeHandle) -> Self {
        let mut err: libc::c_int = 0;
        let mut len = mem::size_of::<libc::c_int>() as libc::socklen_t;
        unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                &mut err as *mut _ as *mut _,
                &mut len,
            );
        }
        ErrorCode(err)
    }

    pub fn from_yield(data: usize) -> Self {
        ErrorCode(data as i32)
    }

    pub fn into_yield(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf: [u8; 256] = unsafe { mem::uninitialized() };
        let msg = unsafe {
            libc::strerror_r(self.0, buf.as_mut_ptr() as *mut i8, buf.len());
            CStr::from_bytes_with_nul_unchecked(&buf).to_str().unwrap()
        };
        write!(f, "{}", msg)
    }
}

/// Not error.
pub const SUCCESS: ErrorCode = ErrorCode(0);

/// Permission denied.
pub const ACCESS_DENIED: ErrorCode = ErrorCode(libc::EACCES);

/// Address family not supported by protocol.
pub const ADDRESS_FAMILY_NOT_SUPPORTED: ErrorCode = ErrorCode(libc::EAFNOSUPPORT);

/// Address already in use.
pub const ADDRESS_IN_USE: ErrorCode = ErrorCode(libc::EADDRINUSE);

/// Transport endpoint is already connected.
pub const ALREADY_CONNECTED: ErrorCode = ErrorCode(libc::EISCONN);

/// Operation already in progress.
pub const ALREADY_STARTED: ErrorCode = ErrorCode(libc::EALREADY);

/// Broken pipe.
pub const BROKEN_PIPE: ErrorCode = ErrorCode(libc::EPIPE);

/// A connection has been aborted.
pub const CONNECTION_ABORTED: ErrorCode = ErrorCode(libc::ECONNABORTED);

/// connection refused.
pub const CONNECTION_REFUSED: ErrorCode = ErrorCode(libc::ECONNREFUSED);

/// Connection reset by peer.
pub const CONNECTION_RESET: ErrorCode = ErrorCode(libc::ECONNRESET);

/// Bad file descriptor.
pub const BAD_DESCRIPTOR: ErrorCode = ErrorCode(libc::EBADF);

/// Bad address.
pub const FAULT: ErrorCode = ErrorCode(libc::EFAULT);

/// No route to host.
pub const HOST_UNREACHABLE: ErrorCode = ErrorCode(libc::EHOSTUNREACH);

/// peration now in progress.
pub const IN_PROGRESS: ErrorCode = ErrorCode(libc::EINPROGRESS);

/// Interrupted system call.
pub const INTERRUPTED: ErrorCode = ErrorCode(libc::EINTR);

/// Invalid argument.
pub const INVALID_ARGUMENT: ErrorCode = ErrorCode(libc::EINVAL);

/// Message to long.
pub const MESSAGE_SIZE: ErrorCode = ErrorCode(libc::EMSGSIZE);

/// The name was too long.
pub const NAME_TOO_LONG: ErrorCode = ErrorCode(libc::ENAMETOOLONG);

/// Network is down.
pub const NETWORK_DOWN: ErrorCode = ErrorCode(libc::ENETDOWN);

/// Network dropped connection on reset.
pub const NETWORK_RESET: ErrorCode = ErrorCode(libc::ENETRESET);

/// Network is unreachable.
pub const NETWORK_UNREACHABLE: ErrorCode = ErrorCode(libc::ENETUNREACH);

/// Too many open files.
pub const NO_DESCRIPTORS: ErrorCode = ErrorCode(libc::EMFILE);

/// No buffer space available.
pub const NO_BUFFER_SPACE: ErrorCode = ErrorCode(libc::ENOBUFS);

/// Cannot allocate memory.
pub const NO_MEMORY: ErrorCode = ErrorCode(libc::ENOMEM);

/// Operation not permitted.
pub const NO_PERMISSION: ErrorCode = ErrorCode(libc::EPERM);

/// Protocol not available.
pub const NO_PROTOCOL_OPTION: ErrorCode = ErrorCode(libc::ENOPROTOOPT);

/// No such device.
pub const NO_SUCH_DEVICE: ErrorCode = ErrorCode(libc::ENODEV);

/// Transport endpoint is not connected.
pub const NOT_CONNECTED: ErrorCode = ErrorCode(libc::ENOTCONN);

/// Socket operation on non-socket.
pub const NOT_SOCKET: ErrorCode = ErrorCode(libc::ENOTSOCK);

/// Operation cancelled.
pub const OPERATION_CANCELED: ErrorCode = ErrorCode(libc::ECANCELED);

/// Operation not supported.
pub const OPERATION_NOT_SUPPORTED: ErrorCode = ErrorCode(libc::EOPNOTSUPP);

/// Cannot send after transport endpoint shutdown.
pub const SHUT_DOWN: ErrorCode = ErrorCode(libc::ESHUTDOWN);

/// Connection timed out.
pub const TIMED_OUT: ErrorCode = ErrorCode(libc::ETIMEDOUT);

/// Resource temporarily unavailable.
pub const TRY_AGAIN: ErrorCode = ErrorCode(libc::EAGAIN);

/// The socket is marked non-blocking and the requested operation would block.
pub const WOULD_BLOCK: ErrorCode = ErrorCode(libc::EWOULDBLOCK);
