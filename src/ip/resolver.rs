use super::{IpEndpoint, IpProtocol};
use crate::ffi;
use crate::{AddressFamily, SocketType, Endpoint, IoContext, Protocol, ResolverError};
use std::ffi::CString;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub struct ResolverQuery {
    node: Option<CString>,
    serv: Option<CString>,
    flags: i32,
    family: AddressFamily,
}

impl ResolverQuery {
    const fn hints(&self, family: AddressFamily, socktype: SocketType, protocol: IpProtocol) -> libc::addrinfo
    {
        libc::addrinfo {
            ai_flags: self.flags,
            ai_family: family.0 as i32,
            ai_socktype: socktype.0 as i32,
            ai_protocol: protocol.0 as i32,
            ai_addrlen: 0,
            ai_addr: ptr::null_mut(),
            ai_canonname: ptr::null_mut(),
            ai_next: ptr::null_mut(),
        }
    }
}


impl From<(&str, &str)> for ResolverQuery {
    fn from((node, serv): (&str, &str)) -> Self {
        ResolverQuery {
            node: Some(CString::new(node).unwrap()),
            serv: Some(CString::new(serv).unwrap()),
            flags: 0,
            family: AddressFamily::UNSPEC,
        }
    }
}


impl From<(&str, u16)> for ResolverQuery {
    fn from((node, port): (&str, u16)) -> Self {
        ResolverQuery {
            node: Some(CString::new(node).unwrap()),
            serv: Some(CString::new(port.to_string()).unwrap()),
            flags: libc::AI_NUMERICSERV,
            family: AddressFamily::UNSPEC,
        }
    }
}


impl From<(IpAddr, u16)> for ResolverQuery {
    fn from((node, port): (IpAddr, u16)) -> Self {
        ResolverQuery {
            node: Some(CString::new(node.to_string()).unwrap()),
            serv: Some(CString::new(port.to_string()).unwrap()),
            flags: libc::AI_NUMERICHOST | libc::AI_NUMERICSERV,
            family: AddressFamily::UNSPEC,
        }
    }
}


impl From<(Ipv4Addr, u16)> for ResolverQuery {
    fn from((node, port): (Ipv4Addr, u16)) -> Self {
        ResolverQuery {
            node: Some(CString::new(node.to_string()).unwrap()),
            serv: Some(CString::new(port.to_string()).unwrap()),
            flags: libc::AI_NUMERICHOST | libc::AI_NUMERICSERV,
            family: AddressFamily::INET,
        }
    }
}


impl From<(Ipv6Addr, u16)> for ResolverQuery {
    fn from((node, port): (Ipv6Addr, u16)) -> Self {
        ResolverQuery {
            node: Some(CString::new(node.to_string()).unwrap()),
            serv: Some(CString::new(port.to_string()).unwrap()),
            flags: libc::AI_NUMERICHOST | libc::AI_NUMERICSERV,
            family: AddressFamily::INET6,
        }
    }
}

pub struct ResolverIter<P> {
    base: *mut libc::addrinfo,
    ai: *mut libc::addrinfo,
    _ctx: IoContext,
    _marker: PhantomData<P>,
}

impl<P> Drop for ResolverIter<P> {
    fn drop(&mut self) {
        ffi::freeaddrinfo(self.base)
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
            let mut sa = MaybeUninit::<Self::Item>::uninit();
            unsafe {
                let dst = sa.as_mut_ptr() as *mut u8;
                let src = (*self.ai).ai_addr as *const u8;
                let len = (*self.ai).ai_addrlen;
                src.copy_to(dst, len as usize);
                self.ai = (*self.ai).ai_next;
                Some(Self::Item::init(sa, len))
            }
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
        let family = match self.pro.family_type() {
            AddressFamily::UNSPEC =>
                query.family,
            AddressFamily::INET if query.family != AddressFamily::INET6 =>
                query.family,
            AddressFamily::INET6 if query.family != AddressFamily::INET =>
                query.family,
            _ =>
                return Err(ResolverError::NOT_SUPPORTED),
        };
        let hints = query.hints(family, self.pro.socket_type(), self.pro.protocol_type());
        let ai = ffi::getaddrinfo(query.node, query.serv, hints)?;
        Ok(ResolverIter {
            base: ai,
            ai: ai,
            _ctx: self.ctx.clone(),
            _marker: PhantomData,
        })
    }
}
