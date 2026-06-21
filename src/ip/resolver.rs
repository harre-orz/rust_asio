use crate::IoContext;
use crate::ip::{IpEndpoint, IpProtocol};
use crate::socket_base::{EndpointRef, Protocol};
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use self::unix::{AddrInfo, AddrInfoIter};
#[cfg(unix)]
pub use self::unix::{ResolverError, ResolverQuery};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows::{AddrInfo, AddrInfoIter, ResolverError, ResolverQuery};

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
    P: Protocol<Endpoint = IpEndpoint<P>, Type = IpProtocol> + 'a,
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

impl<'a, P> Resolved<P>
where
    P: Protocol + 'a,
{
    pub const fn as_ctx(&'a self) -> &'a IoContext {
        &self.ctx
    }
}

impl<'a, P> IntoIterator for &'a Resolved<P>
where
    P: Protocol<Endpoint = IpEndpoint<P>, Type = IpProtocol>,
{
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = ResolvedIter<'a, P>;

    fn into_iter(self) -> Self::IntoIter {
        ResolvedIter {
            ai: self.res.into_iter(),
            _marker: PhantomData,
        }
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
        let f = self.pro.family_type().into();
        let s = self.pro.socket_type().into();
        let p = self.pro.protocol_type().into();
        let res = AddrInfo::new(f, s, p, query.into())?;
        Ok(Resolved {
            ctx: self.ctx.clone(),
            res: res,
            _marker: PhantomData,
        })
    }
}
