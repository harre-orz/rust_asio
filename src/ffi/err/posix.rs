#![allow(dead_code)]

use std::fmt::{Display, Formatter, Result};
use libc;
use errno::{Errno, errno};


// TODO:
const EAI_SERVICE: i32 = 1;
const EAI_SOCKTYPE: i32 = 2;


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct SystemError(Errno);

impl SystemError {
    pub fn last_error() -> Self {
        SystemError(errno())
    }
}

impl Default for SystemError {
    fn default() -> Self {
        SystemError(Errno(0))
    }
}

impl Display for SystemError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.0)
    }
}


/// Permission denied.
pub const ACCESS_DENIED: SystemError = SystemError(Errno(libc::EACCES));

/// Address family not supported by protocol.
pub const ADDRESS_FAMILY_NOT_SUPPORTED: SystemError = SystemError(Errno(libc::EAFNOSUPPORT));

/// Address already in use.
pub const ADDRESS_IN_USE: SystemError = SystemError(Errno(libc::EADDRINUSE));

/// Transport endpoint is already connected.
pub const ALREADY_CONNECTED: SystemError = SystemError(Errno(libc::EISCONN));

/// Operation already in progress.
pub const ALREADY_STARTED: SystemError = SystemError(Errno(libc::EALREADY));

/// Broken pipe.
pub const BROKEN_PIPE: SystemError = SystemError(Errno(libc::EPIPE));

/// A connection has been aborted.
pub const CONNECTION_ABORTED: SystemError = SystemError(Errno(libc::ECONNABORTED));

/// Connection refused.
pub const CONNECTION_REFUSED: SystemError = SystemError(Errno(libc::ECONNREFUSED));

/// Connection reset by peer.
pub const CONNECTION_RESET: SystemError = SystemError(Errno(libc::ECONNRESET));

/// Bad file descriptor.
pub const BAD_DESCRIPTOR: SystemError = SystemError(Errno(libc::EBADF));

/// Bad address.
pub const FAULT: SystemError = SystemError(Errno(libc::EFAULT));

/// No route to host.
pub const HOST_UNREACHABLE: SystemError = SystemError(Errno(libc::EHOSTUNREACH));

/// peration now in progress.
pub const IN_PROGRESS: SystemError = SystemError(Errno(libc::EINPROGRESS));

/// Interrupted system call.
pub const INTERRUPTED: SystemError = SystemError(Errno(libc::EINTR));

/// Invalid argument.
pub const INVALID_ARGUMENT: SystemError = SystemError(Errno(libc::EINVAL));

/// Message to long.
pub const MESSAGE_SIZE: SystemError = SystemError(Errno(libc::EMSGSIZE));

/// The name was too long.
pub const NAME_TOO_LONG: SystemError = SystemError(Errno(libc::ENAMETOOLONG));

/// Network is down.
pub const NETWORK_DOWN: SystemError = SystemError(Errno(libc::ENETDOWN));

/// Network dropped connection on reset.
pub const NETWORK_RESET: SystemError = SystemError(Errno(libc::ENETRESET));

/// Network is unreachable.
pub const NETWORK_UNREACHABLE: SystemError = SystemError(Errno(libc::ENETUNREACH));

/// Too many open files.
pub const NO_DESCRIPTORS: SystemError = SystemError(Errno(libc::EMFILE));

/// No buffer space available.
pub const NO_BUFFER_SPACE: SystemError = SystemError(Errno(libc::ENOBUFS));

/// Cannot allocate memory.
pub const NO_MEMORY: SystemError = SystemError(Errno(libc::ENOMEM));

/// Operation not permitted.
pub const NO_PERMISSION: SystemError = SystemError(Errno(libc::EPERM));

/// Protocol not available.
pub const NO_PROTOCOL_OPTION: SystemError = SystemError(Errno(libc::ENOPROTOOPT));

/// No such device.
pub const NO_SUCH_DEVICE: SystemError = SystemError(Errno(libc::ENODEV));

/// Transport endpoint is not connected.
pub const NOT_CONNECTED: SystemError = SystemError(Errno(libc::ENOTCONN));

/// Socket operation on non-socket.
pub const NOT_SOCKET: SystemError = SystemError(Errno(libc::ENOTSOCK));

/// Operation cancelled.
pub const OPERATION_CANCELED: SystemError = SystemError(Errno(libc::ECANCELED));

/// Operation not supported.
pub const OPERATION_NOT_SUPPORTED: SystemError = SystemError(Errno(libc::EOPNOTSUPP));

/// Cannot send after transport endpoint shutdown.
pub const SHUT_DOWN: SystemError = SystemError(Errno(libc::ESHUTDOWN));

/// Connection timed out.
pub const TIMED_OUT: SystemError = SystemError(Errno(libc::ETIMEDOUT));

/// Resource temporarily unavailable.
pub const TRY_AGAIN: SystemError = SystemError(Errno(libc::EAGAIN));

/// The socket is marked non-blocking and the requested operation would block.
pub const WOULD_BLOCK: SystemError = SystemError(Errno(libc::EWOULDBLOCK));


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct AddrInfoError(Errno);

impl AddrInfoError {
    fn last_error() -> Self {
        AddrInfoError(errno())
    }
}

impl Default for AddrInfoError {
    fn default() -> Self {
        AddrInfoError(Errno(0))
    }
}

impl Display for AddrInfoError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        use std::ffi::CStr;
        write!(f, "{}", unsafe { CStr::from_ptr(libc::gai_strerror((self.0).0)) }.to_str().unwrap())
    }
}


/// The service is not supported for the given socket type.
pub const SERVICE_NOT_FOUND: AddrInfoError = AddrInfoError(Errno(EAI_SERVICE));

/// The socket type is not supported.
pub const SOCKET_TYPE_NOT_SUPPORTED: AddrInfoError = AddrInfoError(Errno(EAI_SOCKTYPE));
