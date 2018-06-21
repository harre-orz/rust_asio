#![allow(dead_code)]

use std::fmt;
use std::io;
use libc::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SystemError(pub i32);

impl SystemError {
    pub fn last_error() -> Self {
        SystemError(unsafe { *__errno_location() })
    }
}

#[cfg(target_os = "macos")]
impl SystemError {
    pub fn from_signal(sig: Signal) -> Self {
        SystemError(-(sig as i32))
    }

    pub fn try_signal(self) -> Result<Signal, Self> {
        if self.0 < 0 {
            Ok(unsafe { mem::transmute(-self.0) })
        } else {
            Err(self)
        }
    }
}

impl Default for SystemError {
    fn default() -> Self {
        SystemError(0)
    }
}

impl fmt::Debug for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::ffi::CStr;
        let mut buf: [u8; 256] = [0; 256];
        let msg = unsafe {
            strerror_r(self.0, buf.as_mut_ptr() as *mut i8, buf.len());
            CStr::from_bytes_with_nul_unchecked(&buf).to_str().unwrap()
        };
        write!(f, "{}", msg)
    }
}

impl From<SystemError> for io::Error {
    fn from(err: SystemError) -> Self {
        io::Error::from_raw_os_error(err.0)
    }
}

/// Permission denied.
pub const ACCESS_DENIED: SystemError = SystemError(EACCES);

/// Address family not supported by protocol.
pub const ADDRESS_FAMILY_NOT_SUPPORTED: SystemError = SystemError(EAFNOSUPPORT);

/// Address already in use.
pub const ADDRESS_IN_USE: SystemError = SystemError(EADDRINUSE);

/// Transport endpoint is already connected.
pub const ALREADY_CONNECTED: SystemError = SystemError(EISCONN);

/// Operation already in progress.
pub const ALREADY_STARTED: SystemError = SystemError(EALREADY);

/// Broken pipe.
pub const BROKEN_PIPE: SystemError = SystemError(EPIPE);

/// A connection has been aborted.
pub const CONNECTION_ABORTED: SystemError = SystemError(ECONNABORTED);

/// connection refused.
pub const CONNECTION_REFUSED: SystemError = SystemError(ECONNREFUSED);

/// Connection reset by peer.
pub const CONNECTION_RESET: SystemError = SystemError(ECONNRESET);

/// Bad file descriptor.
pub const BAD_DESCRIPTOR: SystemError = SystemError(EBADF);

/// Bad address.
pub const FAULT: SystemError = SystemError(EFAULT);

/// No route to host.
pub const HOST_UNREACHABLE: SystemError = SystemError(EHOSTUNREACH);

/// peration now in progress.
pub const IN_PROGRESS: SystemError = SystemError(EINPROGRESS);

/// Interrupted system call.
pub const INTERRUPTED: SystemError = SystemError(EINTR);

/// Invalid argument.
pub const INVALID_ARGUMENT: SystemError = SystemError(EINVAL);

/// Message to long.
pub const MESSAGE_SIZE: SystemError = SystemError(EMSGSIZE);

/// The name was too long.
pub const NAME_TOO_LONG: SystemError = SystemError(ENAMETOOLONG);

/// Network is down.
pub const NETWORK_DOWN: SystemError = SystemError(ENETDOWN);

/// Network dropped connection on reset.
pub const NETWORK_RESET: SystemError = SystemError(ENETRESET);

/// Network is unreachable.
pub const NETWORK_UNREACHABLE: SystemError = SystemError(ENETUNREACH);

/// Too many open files.
pub const NO_DESCRIPTORS: SystemError = SystemError(EMFILE);

/// No buffer space available.
pub const NO_BUFFER_SPACE: SystemError = SystemError(ENOBUFS);

/// Cannot allocate memory.
pub const NO_MEMORY: SystemError = SystemError(ENOMEM);

/// Operation not permitted.
pub const NO_PERMISSION: SystemError = SystemError(EPERM);

/// Protocol not available.
pub const NO_PROTOCOL_OPTION: SystemError = SystemError(ENOPROTOOPT);

/// No such device.
pub const NO_SUCH_DEVICE: SystemError = SystemError(ENODEV);

/// Transport endpoint is not connected.
pub const NOT_CONNECTED: SystemError = SystemError(ENOTCONN);

/// Socket operation on non-socket.
pub const NOT_SOCKET: SystemError = SystemError(ENOTSOCK);

/// Operation cancelled.
pub const OPERATION_CANCELED: SystemError = SystemError(ECANCELED);

/// Operation not supported.
pub const OPERATION_NOT_SUPPORTED: SystemError = SystemError(EOPNOTSUPP);

/// Cannot send after transport endpoint shutdown.
pub const SHUT_DOWN: SystemError = SystemError(ESHUTDOWN);

/// Connection timed out.
pub const TIMED_OUT: SystemError = SystemError(ETIMEDOUT);

/// Resource temporarily unavailable.
pub const TRY_AGAIN: SystemError = SystemError(EAGAIN);

/// The socket is marked non-blocking and the requested operation would block.
pub const WOULD_BLOCK: SystemError = SystemError(EWOULDBLOCK);
