use crate::error::OsError;
use crate::sockaddr::{SockAddrIp, SockLen};
use crate::socket_base::Protocol;
use std::ffi::{CString, OsString};
use std::fmt;
use std::io;
use std::mem::MaybeUninit;
use std::ptr;
use windows_sys::Win32::Networking::WinSock;

/// The getaddrinfo() specified error code.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResolverError {
    err: OsError,
}

/// https://learn.microsoft.com/ja-jp/windows/win32/api/ws2tcpip/nf-ws2tcpip-getaddrinfo
impl ResolverError {
    const fn new(errno: WinSock::WSA_ERROR) -> Self {
        Self {
            err: unsafe { OsError::from_raw(errno) },
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

    pub fn desc(&self) -> OsString {
        self.err.desc()
    }
}

impl fmt::Debug for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ResolverError {{ err = {} ({}) }}",
            self.err,
            self.desc().into_string().unwrap_or_default(),
        )
    }
}

impl Into<io::Error> for ResolverError {
    fn into(self) -> io::Error {
        self.err.into()
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
            flags: WinSock::AI_NUMERICSERV as i32,
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
            flags: (WinSock::AI_NUMERICHOST | WinSock::AI_NUMERICSERV) as i32,
        }
    }
}

pub(crate) struct AddrInfoIter<'a>(Option<&'a WinSock::ADDRINFOA>);

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
            Some((unsafe { &*sa }, ai.ai_addrlen as SockLen))
        } else {
            None
        }
    }
}

pub(crate) struct AddrInfo(pub(super) *mut WinSock::ADDRINFOA);

unsafe impl Send for AddrInfo {}

unsafe impl Sync for AddrInfo {}

impl Drop for AddrInfo {
    fn drop(&mut self) {
        unsafe { WinSock::freeaddrinfo(self.0) }
    }
}

impl AddrInfo {
    pub(crate) fn get<P>(pro: P, query: ResolverQuery) -> Result<Self, ResolverError>
    where
        P: Protocol,
    {
        let node = if query.node.is_empty() {
            ptr::null()
        } else {
            query.node.as_ptr().cast()
        };
        let serv = if query.serv.is_empty() {
            ptr::null()
        } else {
            query.serv.as_ptr().cast()
        };
        let hints = WinSock::ADDRINFOA {
            ai_flags: query.flags,
            ai_family: pro.family_type().into(),
            ai_socktype: pro.socket_type().into(),
            ai_protocol: pro.protocol_type().into(),
            ai_addrlen: 0,
            ai_addr: ptr::null_mut(),
            ai_canonname: ptr::null_mut(),
            ai_next: ptr::null_mut(),
        };
        let mut res = MaybeUninit::<*mut WinSock::ADDRINFOA>::uninit();
        unsafe {
            match WinSock::getaddrinfo(node, serv, &hints, res.as_mut_ptr()) {
                0 => {
                    let res = res.assume_init();
                    Ok(AddrInfo(res))
                }
                err => Err(ResolverError::new(err)),
            }
        }
    }

    pub(crate) const fn iter(&self) -> AddrInfoIter<'_> {
        AddrInfoIter(Some(unsafe { &*self.0 }))
    }
}
