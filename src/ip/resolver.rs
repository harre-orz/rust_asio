use super::{IpEndpoint, IpProtocol};
use crate::error::ResolverError;
use crate::executor::IoContext;
use crate::ffi;
use crate::socket_base::{AddressFamily, Endpoint, Protocol, SocketType};
use std::ffi::CString;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ptr::{self, NonNull};

pub struct ResolverQuery {
    node: CString,
    serv: CString,
    flags: i32,
    family: AddressFamily,
}

impl ResolverQuery {
    fn hints(
        &self,
        family: AddressFamily,
        socktype: SocketType,
        protocol: IpProtocol,
    ) -> libc::addrinfo {
        libc::addrinfo {
            ai_flags: self.flags,
            ai_family: family.into(),
            ai_socktype: socktype.into(),
            ai_protocol: protocol.into(),
            ai_addrlen: 0,
            ai_addr: ptr::null_mut(),
            ai_canonname: ptr::null_mut(),
            ai_next: ptr::null_mut(),
        }
    }
}

impl From<(&str, &str)> for ResolverQuery {
    fn from((node, port): (&str, &str)) -> Self {
        ResolverQuery {
            node: CString::new(node).unwrap(),
            serv: CString::new(port).unwrap(),
            flags: 0,
            family: AddressFamily::UNSPEC,
        }
    }
}

impl From<(&String, &str)> for ResolverQuery {
    fn from((node, port): (&String, &str)) -> Self {
        ResolverQuery {
            node: CString::new(node.as_str()).unwrap(),
            serv: CString::new(port).unwrap(),
            flags: 0,
            family: AddressFamily::UNSPEC,
        }
    }
}

impl From<(String, &str)> for ResolverQuery {
    fn from((node, port): (String, &str)) -> Self {
        ResolverQuery {
            node: CString::new(node.as_str()).unwrap(),
            serv: CString::new(port).unwrap(),
            flags: 0,
            family: AddressFamily::UNSPEC,
        }
    }
}

impl From<(&str, u16)> for ResolverQuery {
    fn from((node, port): (&str, u16)) -> Self {
        ResolverQuery {
            node: CString::new(node).unwrap(),
            serv: CString::new(port.to_string()).unwrap(),
            flags: libc::AI_NUMERICSERV,
            family: AddressFamily::UNSPEC,
        }
    }
}

impl From<(IpAddr, u16)> for ResolverQuery {
    fn from((node, port): (IpAddr, u16)) -> Self {
        ResolverQuery {
            node: CString::new(node.to_string()).unwrap(),
            serv: CString::new(port.to_string()).unwrap(),
            flags: libc::AI_NUMERICHOST | libc::AI_NUMERICSERV,
            family: AddressFamily::UNSPEC,
        }
    }
}

impl From<(Ipv4Addr, u16)> for ResolverQuery {
    fn from((node, port): (Ipv4Addr, u16)) -> Self {
        ResolverQuery {
            node: CString::new(node.to_string()).unwrap(),
            serv: CString::new(port.to_string()).unwrap(),
            flags: libc::AI_NUMERICHOST | libc::AI_NUMERICSERV,
            family: AddressFamily::INET,
        }
    }
}

impl From<(Ipv6Addr, u16)> for ResolverQuery {
    fn from((node, port): (Ipv6Addr, u16)) -> Self {
        ResolverQuery {
            node: CString::new(node.to_string()).unwrap(),
            serv: CString::new(port.to_string()).unwrap(),
            flags: libc::AI_NUMERICHOST | libc::AI_NUMERICSERV,
            family: AddressFamily::INET6,
        }
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
        let family = match self.pro.family_type() {
            AddressFamily::UNSPEC => query.family,
            AddressFamily::INET if query.family != AddressFamily::INET6 => self.pro.family_type(),
            AddressFamily::INET6 if query.family != AddressFamily::INET => self.pro.family_type(),
            _ => return Err(ResolverError::NOT_SUPPORTED_FAMILY),
        };
        let base = ffi::getaddrinfo(
            query.node.as_c_str(),
            query.serv.as_c_str(),
            query.hints(family, self.pro.socket_type(), self.pro.protocol_type()),
        )?;
        Ok(ResolverIter {
            base: base,
            ai: base.as_ptr(),
            _ctx: self.ctx.clone(),
            _marker: PhantomData,
        })
    }
}
