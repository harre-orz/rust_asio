use std::error;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::num::NonZero;

/// The OS specified error code.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OsError {
    #[cfg(unix)]
    errno: NonZero<libc::c_int>,
    #[cfg(windows)]
    errno: NonZero<windows_sys::Win32::Networking::WinSock::WSA_ERROR>,
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
    #[cfg(unix)]
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
            errno: unsafe {
                NonZero::new_unchecked(ffi::last())
            }
        }
    }

    pub fn desc(&self) -> OsString {
        ffi::desc(self.errno.get())
    }
}

impl fmt::Debug for OsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error {{ errno = {} ({}) }}",
            self.errno.get(),
            self.desc().into_string().unwrap_or_default(),
        )
    }
}

impl fmt::Display for OsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.desc().into_string().unwrap_or_default())
    }
}

impl error::Error for OsError {}

impl Into<io::Error> for OsError {
    fn into(self) -> io::Error {
        io::Error::from_raw_os_error(self.errno.get())
    }
}

#[cfg(unix)]
mod ffi {
    use super::*;
    use std::ffi::{CStr, OsStr};
    use std::os::unix::ffi::OsStrExt;

    #[cfg(target_os = "linux")]
    pub(super) unsafe fn last() -> libc::c_int {
        unsafe {
            *libc::__errno_location()
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) unsafe fn last() -> libc::c_int {
        unsafe {
            *libc::__error()
        }
    }

    pub(super) fn desc(errno: i32) -> OsString {
        unsafe {
            let s = CStr::from_ptr(libc::strerror(errno));
            let s = OsStr::from_bytes(s.to_bytes());
            OsString::from(s)
        }
    }

    /// The getaddrinfo() specified error code.
    #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct ResolverError {
        ai_err: NonZero<i32>,
        os_err: Option<OsError>,
    }

    impl ResolverError {
        const fn new(errno: libc::c_int) -> Self {
            Self {
                ai_err: NonZero::new(errno).unwrap(),
                os_err: None,
            }
        }

        pub const TRY_AGAIN: Self = Self::new(libc::EAI_AGAIN);
        pub const BAD_FLAGS: Self = Self::new(libc::EAI_BADFLAGS);
        pub const FAILURE: Self = Self::new(libc::EAI_FAIL);
        pub const NO_MEMORY: Self = Self::new(libc::EAI_MEMORY);
        pub const NO_DATA: Self = Self::new(libc::EAI_NODATA);
        pub const SYSTEM: Self = Self::new(libc::EAI_SYSTEM);
        pub const NOT_SUPPORTED_FAMILY: Self = Self::new(libc::EAI_FAMILY);
        pub const NOT_SUPPORTED_SERVICE: Self = Self::new(libc::EAI_SERVICE);
        pub const NOT_SUPPORTED_SOCKTYPE: Self = Self::new(libc::EAI_SOCKTYPE);

        pub(crate) unsafe fn from_raw(ai_err: i32) -> Self {
            if ai_err == libc::EAI_SYSTEM {
                Self {
                    ai_err: Self::SYSTEM.ai_err,
                    os_err: Some(unsafe { OsError::last() }),
                }
            } else {
                Self {
                    ai_err: NonZero::new(ai_err).unwrap(),
                    os_err: None,
                }
            }
        }

        pub(crate) fn from_os_err(os_err: OsError) -> Self {
            Self {
                ai_err: Self::SYSTEM.ai_err,
                os_err: Some(os_err),
            }
        }

        pub(super) fn desc(&self) -> OsString {
            unsafe {
                let s = libc::gai_strerror(self.ai_err.get());
                let s = CStr::from_ptr(s);
                let s = OsStr::from_bytes(s.to_bytes());
                OsString::from(s)
            }
        }
    }

    impl fmt::Debug for ResolverError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "ResolverError {{ ai_err = {} ({}), os_err = {:?} }}",
                self.ai_err.get(),
                self.desc().into_string().unwrap_or_default(),
                self.os_err
            )
        }
    }

    impl fmt::Display for ResolverError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.desc().into_string().unwrap_or_default())
        }
    }

    impl error::Error for ResolverError {}

    impl Into<io::Error> for ResolverError {
        fn into(self) -> io::Error {
            let errno = if let Some(os_err) = self.os_err {
                os_err.errno.get()
            } else {
                self.ai_err.get()
            };
            io::Error::from_raw_os_error(errno)
        }
    }
}

#[cfg(windows)]
mod ffi {
    use super::*;
    use std::os::windows::ffi::OsStringExt;
    use std::ptr;
    use windows_sys::Win32::Networking::WinSock;

    pub(super) unsafe fn last() -> WinSock::WSA_ERROR {
        WinSock::WSAGetLastError()
    }

    pub(super) fn desc(errno: WinSock::WSA_ERROR) -> OsString {
        use windows_sys::Win32::System::Diagnostics::Debug::{
            FORMAT_MESSAGE_FROM_SYSTEM, FormatMessageW,
        };

        let mut buf = [0; 1024];
        unsafe {
            // https://learn.microsoft.com/ja-jp/windows/win32/api/winbase/nf-winbase-formatmessagew
            let len = FormatMessageW(
                FORMAT_MESSAGE_FROM_SYSTEM,
                ptr::null(),
                errno as u32,
                0,
                buf.as_mut_ptr(),
                buf.len() as u32,
                ptr::null(),
            );
            let buf = &buf[0..len as usize];
            OsString::from_wide(buf)
        }
    }

    /// The getaddrinfo() specified error code.
    #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct ResolverError {
        err: OsError,
    }

    /// https://learn.microsoft.com/ja-jp/windows/win32/api/ws2tcpip/nf-ws2tcpip-getaddrinfo
    impl ResolverError {
        const fn new(errno: WinSock::WSA_ERROR) -> Self {
            Self {
                err: OsError {
                    errno: NonZero::new(errno).unwrap(),
                },
            }
        }

        pub const TRY_AGAIN: Self = Self::new(WinSock::WSATRY_AGAIN);
        pub const BAD_FLAGS: Self = Self::new(WinSock::WSAEINVAL);
        pub const FAILURE: Self = Self::new(WinSock::WSANO_RECOVERY);
        pub const NO_MEMORY: Self = Self::new(WinSock::WSA_NOT_ENOUGH_MEMORY);
        pub const WSANO_DATA: Self = Self::new(WinSock::WSANO_DATA);
        pub const NOT_SUPPORTED_FAMILY: Self = Self::new(WinSock::WSAEAFNOSUPPORT);
        pub const NO_DATA: Self = Self::new(WinSock::WSAHOST_NOT_FOUND);
        pub const NOT_SUPPORTED_SERVICE: Self = Self::new(WinSock::WSATYPE_NOT_FOUND);
        pub const NOT_SUPPORTED_SOCKTYPE: Self = Self::new(WinSock::WSAESOCKTNOSUPPORT);
        //pub const WSANOTINITIALIZED: Self = Self::new(WinSock::WSANOTINITIALIZED);

        pub const fn from_raw(errno: i32) -> Self {
            Self {
                err: OsError {
                    errno: NonZero::new(errno).unwrap(),
                },
            }
        }

        pub const fn from_os_err(err: OsError) -> Self {
            Self { err: err }
        }

        pub(super) fn desc(&self) -> OsString {
            self.err.desc()
        }
    }

    impl fmt::Debug for ResolverError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "ResolverError {{ err = {} ({}) }}",
                self.err.errno.get(),
                self.desc().into_string().unwrap_or_default(),
            )
        }
    }

    impl fmt::Display for ResolverError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.desc().into_string().unwrap_or_default())
        }
    }

    impl std::error::Error for ResolverError {}

    impl Into<io::Error> for ResolverError {
        fn into(self) -> io::Error {
            io::Error::from_raw_os_error(self.err.errno.get())
        }
    }
}

pub use self::ffi::ResolverError;
