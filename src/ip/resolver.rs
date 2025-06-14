use std::marker::PhantomData;
use std::mem;
use std::mem::MaybeUninit;
use crate::addrinfo::ffi::AddrInfo;
pub use crate::addrinfo::ffi::ResolverQuery;
use crate::error::ResolverError;
use crate::IoContext;
use crate::ip::IpEndpoint;
use crate::socket_base::Protocol;

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
        self.ai.next().map(|sa| {
            unsafe { mem::transmute(sa) }
        })
    }
}

unsafe impl<'a, P> Send for ResolverIter<'a, P> {}

pub struct Resolver<P> {
    ctx: IoContext,
    pro: P,
}

impl<P> Resolver<P>
where
    P: Protocol
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
        AddrInfo::new(self.pro, query.into()).map(|ai| {
            ResolverIter {
                ai: ai,
                _ctx: self.ctx.clone(),
                _marker: PhantomData,
            }
        })
    }
}
