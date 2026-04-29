use self::ffi::{AddrInfo, AddrInfoIter};
use crate::IoContext;
use crate::ip::IpEndpoint;
use crate::socket_base::{EndpointRef, Endpoints, Protocol};
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::{error, fmt};

pub use self::ffi::{ResolverError, ResolverQuery};

#[cfg(unix)]
mod ffi {
    use crate::error::OsError;
    use crate::sockaddr::{SockAddrIp, SockLen};
    use crate::socket_base::Protocol;
    use std::ffi::{CStr, CString, OsStr, OsString};
    use std::fmt;
    use std::io;
    use std::mem::MaybeUninit;
    use std::num::NonZero;
    use std::os::unix::ffi::OsStrExt;
    use std::ptr;

    /// The getaddrinfo() specified error code.
    #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct ResolverError {
        ai_err: NonZero<i32>,
        os_err: Option<OsError>,
    }

    impl ResolverError {
        const fn new(errno: libc::c_int) -> Self {
            Self {
                ai_err: unsafe { NonZero::new_unchecked(errno) },
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

        unsafe fn from_raw(ai_err: i32) -> Self {
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

        pub fn desc(&self) -> OsString {
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

    impl Into<io::Error> for ResolverError {
        fn into(self) -> io::Error {
            if let Some(os_err) = self.os_err {
                os_err.into()
            } else {
                io::Error::from_raw_os_error(self.ai_err.get())
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

    pub(crate) struct AddrInfoIter<'a>(Option<&'a libc::addrinfo>);

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

    pub(crate) struct AddrInfo(*mut libc::addrinfo);

    unsafe impl Send for AddrInfo {}

    unsafe impl Sync for AddrInfo {}

    impl Drop for AddrInfo {
        fn drop(&mut self) {
            unsafe { libc::freeaddrinfo(self.0) }
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
                query.node.as_ptr()
            };
            let serv = if query.serv.is_empty() {
                ptr::null()
            } else {
                query.serv.as_ptr()
            };
            let hints = libc::addrinfo {
                ai_flags: query.flags,
                ai_family: pro.family_type(),
                ai_socktype: pro.socket_type(),
                ai_protocol: pro.protocol_type(),
                ai_addrlen: 0,
                ai_addr: ptr::null_mut(),
                ai_canonname: ptr::null_mut(),
                ai_next: ptr::null_mut(),
            };
            let mut res = MaybeUninit::<*mut libc::addrinfo>::uninit();
            unsafe {
                match libc::getaddrinfo(node, serv, &hints, res.as_mut_ptr()) {
                    0 => {
                        let res = res.assume_init();
                        Ok(AddrInfo(res))
                    }
                    err => Err(ResolverError::from_raw(err)),
                }
            }
        }

        pub(crate) fn iter(&self) -> AddrInfoIter<'_> {
            AddrInfoIter(Some(unsafe { &*self.0 }))
        }
    }
}

#[cfg(windows)]
mod ffi {
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
                err: OsError::new(errno),
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

        const fn from_raw(errno: i32) -> Self {
            Self {
                err: OsError::new(errno),
            }
        }

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

    pub(crate) struct AddrInfo(*mut WinSock::ADDRINFOA);

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
                    err => Err(ResolverError::from_raw(err)),
                }
            }
        }

        pub(crate) fn iter(&self) -> AddrInfoIter<'_> {
            AddrInfoIter(Some(unsafe { &*self.0 }))
        }
    }
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.desc().into_string().unwrap_or_default())
    }
}

impl error::Error for ResolverError {}

impl From<(&str, &str)> for ResolverQuery {
    fn from((host, port): (&str, &str)) -> Self {
        Self::from_rr(host, port)
    }
}

impl From<(&String, &str)> for ResolverQuery {
    fn from((host, port): (&String, &str)) -> Self {
        Self::from_rr(host, port)
    }
}

impl From<(String, &str)> for ResolverQuery {
    fn from((host, port): (String, &str)) -> Self {
        Self::from_rr(host, port)
    }
}

impl From<(&str, u16)> for ResolverQuery {
    fn from((host, port): (&str, u16)) -> Self {
        Self::from_rt(host, port)
    }
}

impl From<(IpAddr, u16)> for ResolverQuery {
    fn from((host, port): (IpAddr, u16)) -> Self {
        Self::from_tt(host, port)
    }
}

impl From<(Ipv4Addr, u16)> for ResolverQuery {
    fn from((host, port): (Ipv4Addr, u16)) -> Self {
        Self::from_tt(host, port)
    }
}

impl From<(Ipv6Addr, u16)> for ResolverQuery {
    fn from((host, port): (Ipv6Addr, u16)) -> Self {
        Self::from_tt(host, port)
    }
}

pub struct ResolvedIter<'a, P> {
    ai: AddrInfoIter<'a>,
    _marker: PhantomData<P>,
}

impl<'a, P> Iterator for ResolvedIter<'a, P>
where
    P: Protocol<Endpoint = IpEndpoint<P>> + 'a,
{
    type Item = EndpointRef<'a, P::Endpoint>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ai
            .next()
            .map(|(sa_ref, sa_len)| unsafe { EndpointRef::new_unchecked(sa_ref, sa_len) })
    }
}

pub struct Resolved<P> {
    ctx: IoContext,
    res: AddrInfo,
    _marker: PhantomData<P>,
}

impl<P> Resolved<P>
where
    P: Protocol,
{
    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn iter(&self) -> ResolvedIter<'_, P> {
        ResolvedIter {
            ai: self.res.iter(),
            _marker: PhantomData,
        }
    }
}

impl<'a, P> Endpoints<'a, P> for Resolved<P>
where
    P: Protocol<Endpoint = IpEndpoint<P>> + 'a,
{
    type Iter = ResolvedIter<'a, P>;

    fn endpoints(&'a self) -> Self::Iter {
        self.iter()
    }
}

pub struct Resolver<P> {
    ctx: IoContext,
    pro: P,
}

impl<P> Resolver<P>
where
    P: Protocol,
{
    pub(crate) fn new_priv(ctx: &IoContext, pro: P) -> Self {
        Resolver {
            ctx: ctx.clone(),
            pro,
        }
    }

    pub fn resolve<Q>(&self, query: Q) -> Result<Resolved<P>, ResolverError>
    where
        Q: Into<ResolverQuery>,
    {
        let res = ffi::AddrInfo::get(self.pro, query.into())?;
        Ok(Resolved {
            ctx: self.ctx.clone(),
            res: res,
            _marker: PhantomData,
        })
    }
}
