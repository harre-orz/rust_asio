use std::error;
use std::ffi::CStr;
use std::fmt;
use std::io;
use std::num::NonZero;

/// The OS specified error code.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OsError {
    errno: NonZero<libc::c_int>,
}

impl OsError {
    /// Permission denied.
    pub const ACCESS_DENIED: Self = Self {
        errno: NonZero::new(libc::EACCES).unwrap(),
    };

    /// Address family not supported by protocol.
    pub const ADDRESS_FAMILY_NOT_SUPPORTED: Self = Self {
        errno: NonZero::new(libc::EAFNOSUPPORT).unwrap(),
    };

    /// Address already in use.
    pub const ADDRESS_IN_USE: Self = Self {
        errno: NonZero::new(libc::EADDRINUSE).unwrap(),
    };

    /// Transport endpoint is already connected.
    pub const ALREADY_CONNECTED: Self = Self {
        errno: NonZero::new(libc::EISCONN).unwrap(),
    };

    /// Operation already in progress.
    pub const ALREADY_STARTED: Self = Self {
        errno: NonZero::new(libc::EALREADY).unwrap(),
    };

    /// Broken pipe.
    pub const BROKEN_PIPE: Self = Self {
        errno: NonZero::new(libc::EPIPE).unwrap(),
    };

    /// A connection has been aborted.
    pub const CONNECTION_ABORTED: Self = Self {
        errno: NonZero::new(libc::ECONNABORTED).unwrap(),
    };

    /// connection refused.
    pub const CONNECTION_REFUSED: Self = Self {
        errno: NonZero::new(libc::ECONNREFUSED).unwrap(),
    };

    /// Connection reset by peer.
    pub const CONNECTION_RESET: Self = Self {
        errno: NonZero::new(libc::ECONNRESET).unwrap(),
    };

    /// Bad file descriptor.
    pub const BAD_DESCRIPTOR: Self = Self {
        errno: NonZero::new(libc::EBADF).unwrap(),
    };

    /// Bad address.
    pub const FAULT: Self = Self {
        errno: NonZero::new(libc::EFAULT).unwrap(),
    };

    /// No route to host.
    pub const HOST_UNREACHABLE: Self = Self {
        errno: NonZero::new(libc::EHOSTUNREACH).unwrap(),
    };

    /// peration now in progress.
    pub const IN_PROGRESS: Self = Self {
        errno: NonZero::new(libc::EINPROGRESS).unwrap(),
    };

    /// Interrupted system call.
    pub const INTERRUPTED: Self = Self {
        errno: NonZero::new(libc::EINTR).unwrap(),
    };

    /// Invalid argument.
    pub const INVALID_ARGUMENT: Self = Self {
        errno: NonZero::new(libc::EINVAL).unwrap(),
    };

    /// Message to long.
    pub const MESSAGE_SIZE: Self = Self {
        errno: NonZero::new(libc::EMSGSIZE).unwrap(),
    };

    /// The name was too long.
    pub const NAME_TOO_LONG: Self = Self {
        errno: NonZero::new(libc::ENAMETOOLONG).unwrap(),
    };

    /// Network is down.
    pub const NETWORK_DOWN: Self = Self {
        errno: NonZero::new(libc::ENETDOWN).unwrap(),
    };

    /// Network dropped connection on reset.
    pub const NETWORK_RESET: Self = Self {
        errno: NonZero::new(libc::ENETRESET).unwrap(),
    };

    /// Network is unreachable.
    pub const NETWORK_UNREACHABLE: Self = Self {
        errno: NonZero::new(libc::ENETUNREACH).unwrap(),
    };

    /// Too many open files.
    pub const NO_DESCRIPTORS: Self = Self {
        errno: NonZero::new(libc::EMFILE).unwrap(),
    };

    /// No buffer space available.
    pub const NO_BUFFER_SPACE: Self = Self {
        errno: NonZero::new(libc::ENOBUFS).unwrap(),
    };

    /// Cannot allocate memory.
    pub const NO_MEMORY: Self = Self {
        errno: NonZero::new(libc::ENOMEM).unwrap(),
    };

    /// Operation not permitted.
    pub const NO_PERMISSION: Self = Self {
        errno: NonZero::new(libc::EPERM).unwrap(),
    };

    /// Protocol not available.
    pub const NO_PROTOCOL_OPTION: Self = Self {
        errno: NonZero::new(libc::ENOPROTOOPT).unwrap(),
    };

    /// No such device.
    pub const NO_SUCH_DEVICE: Self = Self {
        errno: NonZero::new(libc::ENODEV).unwrap(),
    };

    /// Transport endpoint is not connected.
    pub const NOT_CONNECTED: Self = Self {
        errno: NonZero::new(libc::ENOTCONN).unwrap(),
    };

    /// Socket operation on non-socket.
    pub const NOT_SOCKET: Self = Self {
        errno: NonZero::new(libc::ENOTSOCK).unwrap(),
    };

    /// Operation cancelled.
    pub const OPERATION_CANCELED: Self = Self {
        errno: NonZero::new(libc::ECANCELED).unwrap(),
    };

    /// Operation not supported.
    pub const OPERATION_NOT_SUPPORTED: Self = Self {
        errno: NonZero::new(libc::EOPNOTSUPP).unwrap(),
    };

    /// Cannot send after transport endpoint shutdown.
    pub const SHUT_DOWN: Self = Self {
        errno: NonZero::new(libc::ESHUTDOWN).unwrap(),
    };

    /// Connection timed out.
    pub const TIMED_OUT: Self = Self {
        errno: NonZero::new(libc::ETIMEDOUT).unwrap(),
    };

    /// Resource temporarily unavailable.
    pub const TRY_AGAIN: Self = Self {
        errno: NonZero::new(libc::EAGAIN).unwrap(),
    };

    /// The socket is marked non-blocking and the requested operation would block.
    pub const WOULD_BLOCK: Self = Self {
        errno: NonZero::new(libc::EWOULDBLOCK).unwrap(),
    };

    /// Returns a last error.
    pub(crate) unsafe fn last() -> Self {
        Self {
            errno: NonZero::new_unchecked(*libc::__errno_location()),
        }
    }

    fn desc(&self) -> String {
        unsafe {
            CStr::from_ptr(libc::strerror(self.errno.get()))
                .to_str()
                .unwrap()
                .to_string()
        }
    }
}

impl fmt::Debug for OsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error {{ errno = {} ({}) }}",
            self.errno.get(),
            self.desc()
        )
    }
}

impl fmt::Display for OsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.desc())
    }
}

impl error::Error for OsError {}

impl Into<io::Error> for OsError {
    fn into(self) -> io::Error {
        io::Error::from_raw_os_error(self.errno.get())
    }
}

/// The getaddrinfo() specified error code.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResolverError {
    ai_err: NonZero<i32>,
    os_err: Option<OsError>,
}

impl ResolverError {
    pub const TRY_AGAIN: Self = Self {
        ai_err: NonZero::new(libc::EAI_AGAIN).unwrap(),
        os_err: None,
    };

    pub const FAILURE: Self = Self {
        ai_err: NonZero::new(libc::EAI_FAIL).unwrap(),
        os_err: None,
    };

    pub const NO_MEMORY: Self = Self {
        ai_err: NonZero::new(libc::EAI_MEMORY).unwrap(),
        os_err: None,
    };

    pub const NO_DATA: Self = Self {
        ai_err: NonZero::new(libc::EAI_NODATA).unwrap(),
        os_err: None,
    };

    pub const SYSTEM: Self = Self {
        ai_err: NonZero::new(libc::EAI_SYSTEM).unwrap(),
        os_err: None,
    };

    pub const NOT_SUPPORTED_FAMILY: Self = Self {
        ai_err: NonZero::new(libc::EAI_FAMILY).unwrap(),
        os_err: None,
    };

    pub const NOT_SUPPORTED_SERVICE: Self = Self {
        ai_err: NonZero::new(libc::EAI_SERVICE).unwrap(),
        os_err: None,
    };

    pub const NOT_SUPPORTED_SOCKTYPE: Self = Self {
        ai_err: NonZero::new(libc::EAI_SOCKTYPE).unwrap(),
        os_err: None,
    };

    pub(crate) unsafe fn from_raw(ai_err: i32) -> Self {
        if ai_err == libc::EAI_SYSTEM {
            Self {
                ai_err: NonZero::new(libc::EAI_SYSTEM).unwrap(),
                os_err: Some(OsError::last()),
            }
        } else {
            Self {
                ai_err: NonZero::new_unchecked(ai_err),
                os_err: None,
            }
        }
    }

    pub(crate) fn from_os_err(os_err: OsError) -> Self {
        Self {
            ai_err: NonZero::new(libc::EAI_SYSTEM).unwrap(),
            os_err: Some(os_err),
        }
    }

    fn desc(&self) -> String {
        unsafe {
            CStr::from_ptr(libc::gai_strerror(self.ai_err.get()))
                .to_str()
                .unwrap()
                .to_string()
        }
    }
}

impl fmt::Debug for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ResolverError {{ ai_err = {} ({}), os_err = {:?} }}",
            self.ai_err.get(),
            self.desc(),
            self.os_err
        )
    }
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(os_err) = &self.os_err {
            write!(f, "{} ({})", self.desc(), os_err.desc())
        } else {
            write!(f, "{}", self.desc())
        }
    }
}

impl error::Error for ResolverError {}

impl Into<io::Error> for ResolverError {
    fn into(self) -> io::Error {
        if let Some(os_err) = self.os_err {
            os_err.into()
        } else {
            io::Error::other(self)
        }
    }
}
