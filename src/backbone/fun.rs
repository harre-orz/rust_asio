use std::mem;
use std::cmp;
use std::hash;
use std::slice;
use libc::memcmp;
use SockAddr;

pub fn sockaddr_eq<E: SockAddr>(lhs: &E, rhs: &E) -> bool {
    lhs.size() == rhs.size() && unsafe { memcmp(
        mem::transmute(lhs.as_sockaddr()),
        mem::transmute(rhs.as_sockaddr()),
        lhs.size())
    } == 0
}

pub fn sockaddr_cmp<E: SockAddr>(lhs: &E, rhs: &E) -> cmp::Ordering {
    let cmp = unsafe {
        memcmp(
            mem::transmute(lhs.as_sockaddr()),
            mem::transmute(rhs.as_sockaddr()),
            cmp::min(lhs.size(), rhs.size())
        )
    };
    if cmp == 0 {
        if lhs.size() < rhs.size() {
            cmp::Ordering::Less
        } else if lhs.size() > rhs.size() {
            cmp::Ordering::Greater
        } else {
            cmp::Ordering::Equal
        }
    } else if cmp < 0 {
        cmp::Ordering::Less
    } else {
        cmp::Ordering::Greater
    }
}

pub fn sockaddr_hash<E: SockAddr, H: hash::Hasher>(ep: &E, state: &mut H) {
    let ptr = ep.as_sockaddr() as *const _ as *const u8;
    let buf = unsafe { slice::from_raw_parts(ptr, ep.size()) };
    state.write(buf);
}
