use std::error;
use std::ffi::CStr;
use std::fmt;
use std::io;
use std::result;

/// The OS specified error code.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Error {
    errno: libc::c_int
}

impl Error {
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


    const fn new(errno: i32) -> Self {
        Self {
            errno: errno,
        }
    }

    /// Returns a last error.
    pub(crate) unsafe fn last() -> Self {
        Self::new(*libc::__errno_location())
    }

    fn desc(&self) -> String {
        let bytes = &mut [0; 256];
        unsafe {
            let _err = libc::strerror_r(self.errno, bytes.as_mut_ptr().cast(), bytes.len());
            debug_assert_eq!(_err, 0);
        }
        CStr::from_bytes_until_nul(bytes)
            .unwrap()
            .to_str()
            .unwrap()
            .into()
    }
}

impl Into<io::Error> for Error {
    fn into(self) -> io::Error {
        io::Error::from_raw_os_error(self.errno)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error {{ {}({}) }}", self.desc(), self.errno)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.desc())
    }
}

impl error::Error for Error {}

pub type Result<T> = result::Result<T, Error>;


/// The getaddrinfo() specified error code.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResolverError {
    ai_err: i32,
    err: Error,
}

impl ResolverError {
    pub const TRY_AGAIN: Self = Self::new(libc::EAI_AGAIN);

    pub const FAILURE: Self = Self::new(libc::EAI_FAIL);

    pub const NO_MEMORY: Self = Self::new(libc::EAI_MEMORY);

    pub const NO_DATA: Self = Self::new(libc::EAI_NODATA);

    pub const SYSTEM: Self = Self::new(libc::EAI_SYSTEM);

    pub const NOT_SUPPORTED: Self = Self::new(libc::EAI_FAMILY);


    const fn new(ai_err: i32) -> Self {
        Self {
            ai_err: ai_err,
            err: Error::new(0),
        }
    }

    pub(crate) unsafe fn from_raw(ai_err: i32) -> Self {
        match ai_err {
            libc::EAI_AGAIN => Self::TRY_AGAIN,
            libc::EAI_FAIL => Self::FAILURE,
            libc::EAI_MEMORY => Self::NO_MEMORY,
            libc::EAI_NODATA => Self::NO_DATA,
            libc::EAI_SYSTEM => Self {
                ai_err: ai_err,
                err: Error::last(),
            },
            //
            libc::EAI_FAMILY => panic!("EAI_FAMILY"),
            libc::EAI_BADFLAGS => panic!("EAI_BADFLAGS"),
            libc::EAI_NONAME => panic!("EAI_NONAME"),
            libc::EAI_SERVICE => panic!("EAI_SERVICE"),
            libc::EAI_SOCKTYPE => panic!("EAI_SOCKTYPE"),
            _ => panic!(),
        }
    }
}

impl fmt::Debug for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ResolverError")
    }
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ResolverError")
    }
}

// impl Into<io::Error> for ResolverError {
//     fn into(self) -> io::Error {
//
//     }
// }

impl error::Error for ResolverError {}
