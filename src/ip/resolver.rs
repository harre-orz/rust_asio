use crate::IoContext;
use crate::error::ResolverError;
use crate::ip::IpEndpoint;
use crate::ip::resolver::ffi::AddrInfo;
use crate::socket_base::Protocol;
use std::ffi::CString;
use std::marker::PhantomData;
use std::mem;
use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub struct ResolverQuery {
    node: CString,
    serv: CString,
    flags: i32,
}

#[cfg(unix)]
pub mod ffi {
    use super::*;
    use crate::sockaddr::ffi::SockAddrIp;
    use std::ffi::CString;
    use std::ptr;

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
                flags: libc::AI_NUMERICSERV,
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
                flags: libc::AI_NUMERICHOST | libc::AI_NUMERICSERV,
            }
        }
    }

    pub struct AddrInfo<'a> {
        res: &'a mut libc::addrinfo,
        ai: *mut libc::addrinfo,
    }

    impl<'a> Drop for AddrInfo<'a> {
        fn drop(&mut self) {
            unsafe { libc::freeaddrinfo(self.res) }
        }
    }

    impl<'a> AddrInfo<'a> {
        pub fn new<P>(pro: P, query: ResolverQuery) -> Result<Self, ResolverError>
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
                ai_family: pro.family_type().into(),
                ai_socktype: pro.socket_type().into(),
                ai_protocol: pro.protocol_type().into(),
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
                        Ok(AddrInfo {
                            res: &mut *res,
                            ai: res,
                        })
                    }
                    err => Err(ResolverError::from_raw(err)),
                }
            }
        }

        pub fn next(&mut self) -> Option<&SockAddrIp> {
            if self.ai.is_null() {
                None
            } else {
                let ai = unsafe { &*self.ai };
                let sa = ai.ai_addr as *const SockAddrIp;
                self.ai = ai.ai_next;
                Some(unsafe { &*sa })
            }
        }
    }
}

#[cfg(windows)]
pub mod ffi {
    use super::*;
    use windows_sys::Win32::Networking::WinSock;

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

    pub struct AddrInfo<'a> {
        res: &'a WinSock::ADDRINFOA,
        ai: *mut WinSock::ADDRINFOA,
    }

    impl<'a> Drop for AddrInfo<'a> {
        fn drop(&mut self) {
            unsafe { WinSock::freeaddrinfo(self.res) }
        }
    }

    impl<'a> AddrInfo<'a> {
        pub fn new<P>(pro: P, query: ResolverQuery) -> Result<Self, ResolverError>
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
                        Ok(AddrInfo {
                            res: unsafe { &*res },
                            ai: res,
                        })
                    }
                    err => Err(ResolverError::from_raw(err)),
                }
            }
        }

        pub fn next(&mut self) -> Option<&'a SockAddrIp> {
            if self.ai.is_null() {
                None
            } else {
                let ai = unsafe { &*self.ai };
                let sa = ai.ai_addr as *const SockAddrIp;
                self.ai = ai.ai_next;
                Some(unsafe { &*sa })
            }
        }
    }
}

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

pub struct ResolverIter<'a, P> {
    ai: AddrInfo<'a>,
    _ctx: IoContext,
    _marker: PhantomData<P>,
}

impl<'a, P> Iterator for ResolverIter<'a, P>
where
    P: Protocol + 'a,
{
    type Item = &'a IpEndpoint<P>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ai.next().map(|sa| unsafe { mem::transmute(sa) })
    }
}

unsafe impl<'a, P> Send for ResolverIter<'a, P> {}

pub struct Resolver<P> {
    ctx: IoContext,
    pro: P,
}

impl<P> Resolver<P>
where
    P: Protocol,
{
    pub(crate) fn new_priv(ctx: &IoContext, pro: P) -> Self {
        Self {
            ctx: ctx.clone(),
            pro: pro,
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn protocol(&self) -> P {
        self.pro
    }

    pub fn resolve<Q>(&self, query: Q) -> Result<ResolverIter<P>, ResolverError>
    where
        Q: Into<ResolverQuery>,
    {
        AddrInfo::new(self.pro, query.into()).map(|ai| ResolverIter {
            ai: ai,
            _ctx: self.ctx.clone(),
            _marker: PhantomData,
        })
    }
}
