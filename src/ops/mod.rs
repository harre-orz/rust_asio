use std::mem;
use std::cmp;
use libc;

mod unix;
pub use self::unix::*;

pub mod async;


pub fn str2c_char(src: &str, dst: &mut [c_char]) {
    let len = cmp::min(dst.len()-1, src.len());
    for (dst, src) in dst.iter_mut().zip(src.chars()) {
        *dst = src as c_char;
    };
    dst[len] = '\0' as c_char;
}

pub fn c_char2string(src: &[c_char]) -> String {
    let mut s = String::new();
    for c in src {
        if *c == 0 {
            break;
        }
        s.push((*c as u8) as char);
    }
    s
}

pub fn raw_sockaddr_eq<T: AsRawSockAddr>(lhs: &T, rhs: &T) -> bool {
    unsafe {
        libc::memcmp(
            mem::transmute(lhs.as_raw_sockaddr()),
            mem::transmute(rhs.as_raw_sockaddr()),
            lhs.raw_socklen() as usize
        ) == 0 }
}

pub fn raw_sockaddr_cmp<T: AsRawSockAddr>(lhs: &T, rhs: &T) -> cmp::Ordering {
    match unsafe {
        libc::memcmp(
            mem::transmute(lhs.as_raw_sockaddr()),
            mem::transmute(rhs.as_raw_sockaddr()),
            lhs.raw_socklen() as usize
        ) }
    {
        0 => cmp::Ordering::Equal,
        x if x < 0 => cmp::Ordering::Less,
        _ => cmp::Ordering::Greater,
    }
}
