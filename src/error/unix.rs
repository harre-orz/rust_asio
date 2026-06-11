use std::ffi::{CStr};
use std::mem::MaybeUninit;
use std::num::NonZero;
use std::{fmt, mem};

/// The OS specified error code.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OsError(NonZero<libc::c_int>);

impl OsError {
    const fn new(errno: libc::c_int) -> Self {
        Self(NonZero::new(errno).unwrap())
    }

    /// Permission denied.
    pub const ACCESS_DENIED: Self = Self::new(libc::EACCES);

    /// Address family not supported by protocol.
    pub const ADDRESS_FAMILY_NOT_SUPPORTED: Self = Self::new(libc::EAFNOSUPPORT);

    /// Address already in use.
    pub const ADDRESS_IN_USE: Self = Self::new(libc::EADDRINUSE);

    /// Transport endpoint is already connected.
    pub const ALREADY_CONNECTED: Self = Self::new(libc::EISCONN);

    /// Operation already in progress.
    pub const ALREADY_STARTED: Self = Self::new(libc::EALREADY);

    /// Broken pipe.
    pub const BROKEN_PIPE: Self = Self::new(libc::EPIPE);

    /// A connection has been aborted.
    pub const CONNECTION_ABORTED: Self = Self::new(libc::ECONNABORTED);

    /// connection refused.
    pub const CONNECTION_REFUSED: Self = Self::new(libc::ECONNREFUSED);

    /// Connection reset by peer.
    pub const CONNECTION_RESET: Self = Self::new(libc::ECONNRESET);

    /// Bad file descriptor.
    pub const BAD_DESCRIPTOR: Self = Self::new(libc::EBADF);

    /// Bad address.
    pub const FAULT: Self = Self::new(libc::EFAULT);

    /// No route to host.
    pub const HOST_UNREACHABLE: Self = Self::new(libc::EHOSTUNREACH);

    /// peration now in progress.
    pub const IN_PROGRESS: Self = Self::new(libc::EINPROGRESS);

    /// Interrupted system call.
    pub const INTERRUPTED: Self = Self::new(libc::EINTR);

    /// Invalid argument.
    pub const INVALID_ARGUMENT: Self = Self::new(libc::EINVAL);

    /// Message to long.
    pub const MESSAGE_SIZE: Self = Self::new(libc::EMSGSIZE);

    /// The name was too long.
    pub const NAME_TOO_LONG: Self = Self::new(libc::ENAMETOOLONG);

    /// Network is down.
    pub const NETWORK_DOWN: Self = Self::new(libc::ENETDOWN);

    /// Network dropped connection on reset.
    pub const NETWORK_RESET: Self = Self::new(libc::ENETRESET);

    /// Network is unreachable.
    pub const NETWORK_UNREACHABLE: Self = Self::new(libc::ENETUNREACH);

    /// Too many open files.
    pub const NO_DESCRIPTORS: Self = Self::new(libc::EMFILE);

    /// No buffer space available.
    pub const NO_BUFFER_SPACE: Self = Self::new(libc::ENOBUFS);

    /// Cannot allocate memory.
    pub const NO_MEMORY: Self = Self::new(libc::ENOMEM);

    /// Operation not permitted.
    pub const NO_PERMISSION: Self = Self::new(libc::EPERM);

    /// Protocol not available.
    pub const NO_PROTOCOL_OPTION: Self = Self::new(libc::ENOPROTOOPT);

    /// No such device.
    pub const NO_SUCH_DEVICE: Self = Self::new(libc::ENODEV);

    /// Transport endpoint is not connected.
    pub const NOT_CONNECTED: Self = Self::new(libc::ENOTCONN);

    /// Socket operation on non-socket.
    pub const NOT_SOCKET: Self = Self::new(libc::ENOTSOCK);

    /// Operation cancelled.
    pub const OPERATION_CANCELED: Self = Self::new(libc::ECANCELED);

    /// Operation not supported.
    pub const OPERATION_NOT_SUPPORTED: Self = Self::new(libc::EOPNOTSUPP);

    /// Cannot send after transport endpoint shutdown.
    pub const SHUT_DOWN: Self = Self::new(libc::ESHUTDOWN);

    /// Connection timed out.
    pub const TIMED_OUT: Self = Self::new(libc::ETIMEDOUT);

    /// Resource temporarily unavailable.
    pub const TRY_AGAIN: Self = Self::new(libc::EAGAIN);

    /// The socket is marked non-blocking and the requested operation would block.
    pub const WOULD_BLOCK: Self = Self::new(libc::EWOULDBLOCK);

    #[cfg(target_os = "linux")]
    pub(crate) unsafe fn last() -> Self {
        unsafe {
            let errno = *libc::__errno_location();
            mem::transmute(errno)
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) unsafe fn last() -> Self {
        unsafe {
            let errno = *libc::__error();
            mem::transmute(errno)
        }
    }

    /// Returns OS specified error number.
    pub fn code(&self) -> libc::c_int {
        self.0.get()
    }
}

impl fmt::Display for OsError {
    /// Returns a string describing error code.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        const BUF_LEN: usize = 256;

        let mut buf: [MaybeUninit<_>; BUF_LEN] = [const { MaybeUninit::uninit() }; BUF_LEN];
        unsafe {
            match libc::strerror_r(self.0.get(), buf[0].as_mut_ptr(), buf.len()) {
                0 => {
                    let buf = mem::transmute::<_, [_; BUF_LEN]>(buf);
                    write!(f, "{}", CStr::from_ptr(buf.as_ptr()).to_string_lossy())
                }
                _ => {
                    write!(f, "Unknown error {}", self.0.get())
                }
            }
        }
    }
}
