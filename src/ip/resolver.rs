use super::{IpEndpoint, IpProtocol};
use crate::error::ResolverError;
use crate::executor::IoContext;
use crate::ffi;
use crate::socket_base::{Endpoint, Protocol};
use std::ffi::CString;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ptr::{self, NonNull};

pub struct ResolverQuery {
    node: CString,
    serv: CString,
    flags: i32,
}

impl ResolverQuery {
    fn hints<P>(&self, pro: P) -> libc::addrinfo
    where
        P: Protocol,
    {
        libc::addrinfo {
            ai_flags: self.flags,
            ai_family: pro.family_type().into(),
            ai_socktype: pro.socket_type().into(),
            ai_protocol: pro.protocol_type().into(),
            ai_addrlen: 0,
            ai_addr: ptr::null_mut(),
            ai_canonname: ptr::null_mut(),
            ai_next: ptr::null_mut(),
        }
    }

    fn from_rr<T, U>(host: T, port: U) -> Self
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

    fn from_rt<T, U>(host: T, port: U) -> Self
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

    fn from_tt<T, U>(host: T, port: U) -> Self
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

pub struct ResolverIter<P> {
    base: NonNull<libc::addrinfo>,
    ai: *mut libc::addrinfo,
    _ctx: IoContext,
    _marker: PhantomData<P>,
}

unsafe impl<P> Send for ResolverIter<P> {}

impl<P> Drop for ResolverIter<P> {
    fn drop(&mut self) {
        unsafe { ffi::freeaddrinfo(self.base) }
    }
}

impl<P> Iterator for ResolverIter<P>
where
    P: Protocol,
{
    type Item = IpEndpoint<P>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ai.is_null() {
            None
        } else {
            let ai = unsafe { *self.ai };
            self.ai = ai.ai_next;
            let src = ai.ai_addr as *const u8;
            let mut sa = MaybeUninit::<Self::Item>::uninit();
            let dst = sa.as_mut_ptr() as *mut u8;
            unsafe { src.copy_to(dst, ai.ai_addrlen as usize) };
            Some(unsafe { Self::Item::init(sa, ai.ai_addrlen) })
        }
    }
}

pub struct Resolver<P> {
    ctx: IoContext,
    pro: P,
}

impl<P> Resolver<P>
where
    P: Protocol<Type = IpProtocol>,
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
        let query = query.into();
        let base = ffi::getaddrinfo(
            query.node.as_c_str(),
            query.serv.as_c_str(),
            query.hints(self.pro),
        )?;
        Ok(ResolverIter {
            base: base,
            ai: base.as_ptr(),
            _ctx: self.ctx.clone(),
            _marker: PhantomData,
        })
    }
}
