//

use super::IpEndpoint;
use error::ErrorCode;
use executor::{IoContext};
use libc;
use socket_base::Protocol;
use std::ffi::CString;
use std::io;
use std::iter::Iterator;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr;

pub struct ResolverQuery(CString);

impl ResolverQuery {
    fn as_ptr(&self) -> *const libc::c_char {
        self.0.as_ptr()
    }
}

impl<T> From<T> for ResolverQuery
where
    T: AsRef<str>,
{
    fn from(name: T) -> Self {
        let name: Vec<u8> = name
            .as_ref()
            .as_bytes()
            .iter()
            .map(|x| *x)
            .filter(|x| *x != 0)
            .collect();
        ResolverQuery(unsafe { CString::from_vec_unchecked(name) })
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
    pub fn new(ctx: &IoContext, pro: P) -> Self {
        Resolver {
            ctx: ctx.clone(),
            pro: pro,
        }
    }

    pub fn as_ctx(&self) -> &IoContext {
        &self.ctx
    }

    pub fn addrinfo<Q>(&self, host: Q, port: u16, flags: i32) -> io::Result<ResolverIter<P>>
    where
        Q: Into<ResolverQuery>,
    {
        let host = host.into();
        let node = host.as_ptr();
        let hints = libc::addrinfo {
            ai_family: self.pro.family_type(),
            ai_socktype: self.pro.socket_type(),
            ai_protocol: self.pro.protocol_type(),
            ai_flags: flags,
            ai_canonname: ptr::null_mut(),
            ai_addrlen: 0,
            ai_addr: ptr::null_mut(),
            ai_next: ptr::null_mut(),
        };
        let mut base = MaybeUninit::<*mut libc::addrinfo>::uninit();
        let err = unsafe { libc::getaddrinfo(node, ptr::null(), &hints, base.as_mut_ptr()) };
        let base = unsafe { base.assume_init() };
        if err == 0 {
            Ok(ResolverIter {
                port: port,
                ai: base,
                base: base,
                _marker: PhantomData,
            })
        } else {
            Err(ErrorCode::last_error().into())
        }
    }
}

pub struct ResolverIter<P> {
    port: u16,
    ai: *mut libc::addrinfo,
    base: *mut libc::addrinfo,
    _marker: PhantomData<P>,
}

impl<P> ResolverIter<P> where P: Protocol {}

impl<P> Drop for ResolverIter<P> {
    fn drop(&mut self) {
        unsafe { libc::freeaddrinfo(self.base) }
    }
}

impl<P> Iterator for ResolverIter<P>
where
    P: Protocol,
{
    type Item = IpEndpoint<P>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.ai.is_null() {
            unsafe {
                let sa = (*self.ai).ai_addr;
                let ep = match (*sa).sa_family as i32 {
                    libc::AF_INET => {
                        let sin = sa as *const libc::sockaddr_in;
                        IpEndpoint::v4((*sin).sin_addr.into(), self.port)
                    }
                    libc::AF_INET6 => {
                        let sin6 = sa as *const libc::sockaddr_in6;
                        IpEndpoint::v6((*sin6).sin6_addr.into(), self.port)
                    }
                    _ => {
                        self.ai = (*self.ai).ai_next;
                        continue;
                    }
                };
                self.ai = (*self.ai).ai_next;
                return Some(ep);
            }
        }
        None
    }
}
