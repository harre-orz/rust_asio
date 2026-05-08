use std::error;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::result;

/// The OS specified error code.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OsError {
    #[cfg(unix)]
    errno: libc::c_int,
    #[cfg(windows)]
    errno: windows_sys::Win32::Networking::WinSock::WSA_ERROR,
}

impl OsError {
    #[cfg(unix)]
    const fn new_impl(errno: libc::c_int) -> Self {
        Self { errno: errno }
    }
    #[cfg(windows)]
    pub(crate) const fn new_impl(
        errno: windows_sys::Win32::Networking::WinSock::WSA_ERROR,
    ) -> Self {
        Self { errno: errno }
    }

    /// Permission denied.
    pub const ACCESS_DENIED: Self = Self::new_impl(libc::EACCES);

    /// Address family not supported by protocol.
    pub const ADDRESS_FAMILY_NOT_SUPPORTED: Self = Self::new_impl(libc::EAFNOSUPPORT);

    /// Address already in use.
    pub const ADDRESS_IN_USE: Self = Self::new_impl(libc::EADDRINUSE);

    /// Transport endpoint is already connected.
    pub const ALREADY_CONNECTED: Self = Self::new_impl(libc::EISCONN);

    /// Operation already in progress.
    pub const ALREADY_STARTED: Self = Self::new_impl(libc::EALREADY);

    /// Broken pipe.
    pub const BROKEN_PIPE: Self = Self::new_impl(libc::EPIPE);

    /// A connection has been aborted.
    pub const CONNECTION_ABORTED: Self = Self::new_impl(libc::ECONNABORTED);

    /// connection refused.
    pub const CONNECTION_REFUSED: Self = Self::new_impl(libc::ECONNREFUSED);

    /// Connection reset by peer.
    pub const CONNECTION_RESET: Self = Self::new_impl(libc::ECONNRESET);

    /// Bad file descriptor.
    pub const BAD_DESCRIPTOR: Self = Self::new_impl(libc::EBADF);

    /// Bad address.
    pub const FAULT: Self = Self::new_impl(libc::EFAULT);

    /// No route to host.
    pub const HOST_UNREACHABLE: Self = Self::new_impl(libc::EHOSTUNREACH);

    /// peration now in progress.
    pub const IN_PROGRESS: Self = Self::new_impl(libc::EINPROGRESS);

    /// Interrupted system call.
    pub const INTERRUPTED: Self = Self::new_impl(libc::EINTR);

    /// Invalid argument.
    pub const INVALID_ARGUMENT: Self = Self::new_impl(libc::EINVAL);

    /// Message to long.
    pub const MESSAGE_SIZE: Self = Self::new_impl(libc::EMSGSIZE);

    /// The name was too long.
    pub const NAME_TOO_LONG: Self = Self::new_impl(libc::ENAMETOOLONG);

    /// Network is down.
    pub const NETWORK_DOWN: Self = Self::new_impl(libc::ENETDOWN);

    /// Network dropped connection on reset.
    pub const NETWORK_RESET: Self = Self::new_impl(libc::ENETRESET);

    /// Network is unreachable.
    pub const NETWORK_UNREACHABLE: Self = Self::new_impl(libc::ENETUNREACH);

    /// Too many open files.
    pub const NO_DESCRIPTORS: Self = Self::new_impl(libc::EMFILE);

    /// No buffer space available.
    pub const NO_BUFFER_SPACE: Self = Self::new_impl(libc::ENOBUFS);

    /// Cannot allocate memory.
    pub const NO_MEMORY: Self = Self::new_impl(libc::ENOMEM);

    /// Operation not permitted.
    pub const NO_PERMISSION: Self = Self::new_impl(libc::EPERM);

    /// Protocol not available.
    pub const NO_PROTOCOL_OPTION: Self = Self::new_impl(libc::ENOPROTOOPT);

    /// No such device.
    pub const NO_SUCH_DEVICE: Self = Self::new_impl(libc::ENODEV);

    /// Transport endpoint is not connected.
    pub const NOT_CONNECTED: Self = Self::new_impl(libc::ENOTCONN);

    /// Socket operation on non-socket.
    pub const NOT_SOCKET: Self = Self::new_impl(libc::ENOTSOCK);

    /// Operation cancelled.
    pub const OPERATION_CANCELED: Self = Self::new_impl(libc::ECANCELED);

    /// Operation not supported.
    pub const OPERATION_NOT_SUPPORTED: Self = Self::new_impl(libc::EOPNOTSUPP);

    /// Cannot send after transport endpoint shutdown.
    #[cfg(unix)]
    pub const SHUT_DOWN: Self = Self::new_impl(libc::ESHUTDOWN);

    /// Connection timed out.
    pub const TIMED_OUT: Self = Self::new_impl(libc::ETIMEDOUT);

    /// Resource temporarily unavailable.
    pub const TRY_AGAIN: Self = Self::new_impl(libc::EAGAIN);

    /// The socket is marked non-blocking and the requested operation would block.
    pub const WOULD_BLOCK: Self = Self::new_impl(libc::EWOULDBLOCK);

    /// Returns a last error.
    ///
    /// # Example
    ///
    /// ```
    /// use asyncio::error::OsError;
    ///
    /// // This 'err' is an indeterminate value if no error occurs in the system call.
    /// let _err = unsafe { OsError::last() };
    /// ```
    pub unsafe fn last() -> Self {
        unsafe {
            #[cfg(target_os = "linux")]
            let errno = *libc::__errno_location();
            #[cfg(target_os = "macos")]
            let errno = *libc::__error();
            #[cfg(target_os = "windows")]
            let errno = windows_sys::Win32::Networking::WinSock::WSAGetLastError();
            Self::new_impl(errno)
        }
    }

    #[cfg(unix)]
    pub fn desc_impl(&self) -> OsString {
        use std::ffi::{CStr, OsStr};
        use std::os::unix::ffi::OsStrExt;

        unsafe {
            let s = CStr::from_ptr(libc::strerror(self.errno));
            let s = OsStr::from_bytes(s.to_bytes());
            OsString::from(s)
        }
    }

    #[cfg(windows)]
    fn desc_impl(&self) -> OsString {
        use std::os::windows::ffi::OsStringExt;
        use std::ptr;
        use windows_sys::Win32::System::Diagnostics::Debug::{
            FORMAT_MESSAGE_FROM_SYSTEM, FormatMessageW,
        };

        let mut buf = [0; 1024];
        unsafe {
            // https://learn.microsoft.com/ja-jp/windows/win32/api/winbase/nf-winbase-formatmessagew
            let len = FormatMessageW(
                FORMAT_MESSAGE_FROM_SYSTEM,
                ptr::null(),
                self.errno.get() as u32,
                0,
                buf.as_mut_ptr(),
                buf.len() as u32,
                ptr::null(),
            );
            let buf = &buf[0..len as usize];
            OsString::from_wide(buf)
        }
    }

    /// Returns a string describing error code.
    pub fn desc(&self) -> OsString {
        self.desc_impl()
    }
}

impl fmt::Debug for OsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error {{ errno = {} ({}) }}",
            self.errno,
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
        io::Error::from_raw_os_error(self.errno)
    }
}

pub type Result<T> = result::Result<T, OsError>;
