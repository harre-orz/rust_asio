use crate::error::OsError;
use crate::sockaddr::{SockAddrIp, SockLen};
use std::ffi::{CStr, CString};
use std::io;
use std::mem::MaybeUninit;
use std::num::NonZero;
use std::ptr;
use std::ptr::NonNull;
use std::{error, fmt, mem};

fn gai_strerror(errno: i32) -> &'static CStr {
    unsafe { CStr::from_ptr(libc::gai_strerror(errno)) }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum Inner {
    Ai(NonZero<i32>),
    Os(OsError),
}

/// The getaddrinfo() specified error code.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResolverError(Inner);

impl ResolverError {
    const fn new(errno: libc::c_int) -> Self {
        Self(Inner::Ai(NonZero::new(errno).unwrap()))
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

    unsafe fn from_raw(err: i32) -> Self {
        if err == libc::EAI_SYSTEM {
            Self(Inner::Os(unsafe { OsError::last() }))
        } else {
            Self(Inner::Ai(unsafe { mem::transmute(err) }))
        }
    }
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            Inner::Ai(err) => {
                let s = gai_strerror(err.get());
                write!(f, "{}", s.to_string_lossy())
            }
            Inner::Os(err) => unsafe {
                let s = gai_strerror(libc::EAI_SYSTEM);
                write!(f, "{} ({})", s.to_string_lossy(), err)
            },
        }
    }
}

impl error::Error for ResolverError {}

impl Into<io::Error> for ResolverError {
    fn into(self) -> io::Error {
        match self.0 {
            Inner::Ai(err) => {
                let s = gai_strerror(err.get());
                io::Error::new(io::ErrorKind::Other, s.to_string_lossy())
            }
            Inner::Os(err) => err.into(),
        }
    }
}

pub struct ResolverQuery {
    node: CString,
    serv: CString,
    flags: i32,
}

impl ResolverQuery {
    pub(super) fn from_rr<T, U>(host: T, port: U) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        Self {
            node: CString::new(host.as_ref()).unwrap(),
            serv: CString::new(port.as_ref()).unwrap(),
            flags: 0,
        }
    }

    pub(super) fn from_rt<T, U>(host: T, port: U) -> Self
    where
        T: AsRef<str>,
        U: ToString,
    {
        Self {
            node: CString::new(host.as_ref()).unwrap(),
            serv: CString::new(port.to_string()).unwrap(),
            flags: libc::AI_NUMERICSERV as i32,
        }
    }

    pub(super) fn from_tt<T, U>(host: T, port: U) -> Self
    where
        T: ToString,
        U: ToString,
    {
        Self {
            node: CString::new(host.to_string()).unwrap(),
            serv: CString::new(port.to_string()).unwrap(),
            flags: (libc::AI_NUMERICHOST | libc::AI_NUMERICSERV) as i32,
        }
    }
}

pub(super) struct AddrInfoIter<'a>(Option<&'a libc::addrinfo>);

unsafe impl<'a> Send for AddrInfoIter<'a> {}

unsafe impl<'a> Sync for AddrInfoIter<'a> {}

impl<'a> Iterator for AddrInfoIter<'a> {
    type Item = (&'a SockAddrIp, SockLen);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ai) = self.0.take() {
            let ai_next = ai.ai_next;
            if !ai_next.is_null() {
                self.0 = Some(unsafe { &*ai_next });
            }
            let sa = ai.ai_addr as *const SockAddrIp;
            debug_assert!(ai.ai_addrlen as usize <= size_of::<SockAddrIp>());
            Some((unsafe { &*sa }, ai.ai_addrlen))
        } else {
            None
        }
    }
}

pub(super) struct AddrInfo {
    ai: NonNull<libc::addrinfo>,
}

unsafe impl Send for AddrInfo {}

unsafe impl Sync for AddrInfo {}

impl Drop for AddrInfo {
    fn drop(&mut self) {
        unsafe { libc::freeaddrinfo(self.ai.as_ptr()) }
    }
}

impl AddrInfo {
    pub fn new(
        family: i32,
        socktype: i32,
        protocol: i32,
        query: ResolverQuery,
    ) -> Result<Self, ResolverError> {
        let node = if query.node.is_empty() {
            ptr::null()
        } else {
            query.node.as_ptr()
        };
        let serv = if query.serv.is_empty() {
            ptr::null()
        } else {
            query.serv.as_ptr()
        };
        let hints = libc::addrinfo {
            ai_flags: query.flags,
            ai_family: family,
            ai_socktype: socktype,
            ai_protocol: protocol,
            ai_addrlen: 0,
            ai_addr: ptr::null_mut(),
            ai_canonname: ptr::null_mut(),
            ai_next: ptr::null_mut(),
        };
        let mut ai = MaybeUninit::<*mut libc::addrinfo>::uninit();
        unsafe {
            match libc::getaddrinfo(node, serv, &hints, ai.as_mut_ptr()) {
                0 => {
                    let ai = ai.assume_init();
                    Ok(AddrInfo {
                        ai: NonNull::new_unchecked(ai),
                    })
                }
                err => Err(ResolverError::from_raw(err)),
            }
        }
    }
}

impl<'a> IntoIterator for &'a AddrInfo {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = AddrInfoIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AddrInfoIter(Some(unsafe { self.ai.as_ref() }))
    }
}
