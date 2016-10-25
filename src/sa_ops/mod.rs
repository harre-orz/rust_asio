use std::mem;
use std::slice;
use std::cmp::{self, Ordering};
use std::hash::Hasher;
use libc::{self, sockaddr, sockaddr_in, sockaddr_in6, sockaddr_storage, sockaddr_un};
use traits::{SockAddr};

pub trait SockAddrTrait : Copy {
}

impl SockAddrTrait for sockaddr {
}

impl SockAddrTrait for sockaddr_in {
}

impl SockAddrTrait for sockaddr_in6 {
}

impl SockAddrTrait for sockaddr_storage {
}

impl SockAddrTrait for sockaddr_un {
}

pub fn sockaddr_eq<E>(lhs: &E, rhs: &E) -> bool
    where E: SockAddr,
{
    lhs.size() == rhs.size() && unsafe { libc::memcmp(
        mem::transmute(lhs.as_sockaddr()),
        mem::transmute(rhs.as_sockaddr()),
        lhs.size())
    } == 0
}

pub fn sockaddr_cmp<E>(lhs: &E, rhs: &E) -> Ordering
    where E: SockAddr,
{
    match unsafe {
        libc::memcmp(
            mem::transmute(lhs.as_sockaddr()),
            mem::transmute(rhs.as_sockaddr()),
            cmp::min(lhs.size(), rhs.size())
        )
    }.cmp(&0) {
        Ordering::Equal => lhs.size().cmp(&rhs.size()),
        cmp => cmp,
    }
}

pub fn sockaddr_hash<E, H>(ep: &E, state: &mut H)
    where E: SockAddr,
          H: Hasher,
{
    let ptr = ep.as_sockaddr() as *const _ as *const u8;
    let buf = unsafe { slice::from_raw_parts(ptr, ep.size()) };
    state.write(buf);
}

#[cfg(target_os = "linux")] mod linux;
#[cfg(target_os = "linux")] pub use self::linux::*;

#[cfg(target_os = "macos")] mod bsd;
#[cfg(target_os = "macos")] pub use self::bsd::*;
