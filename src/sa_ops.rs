use std::mem;
use std::cmp::{self, Ordering};
use std::hash::Hasher;
use std::slice;
use libc;
use traits::{SockAddr};

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
