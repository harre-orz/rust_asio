use crate::error::ResolverError;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr;

pub fn getaddrinfo(
    node: &CStr,
    serv: &CStr,
    hints: libc::addrinfo,
) -> Result<*mut libc::addrinfo, ResolverError> {
    let mut base = MaybeUninit::<*mut libc::addrinfo>::uninit();
    unsafe {
        let node = if node.is_empty() {
            ptr::null()
        } else {
            node.as_ptr()
        };
        let serv = if serv.is_empty() {
            ptr::null()
        } else {
            serv.as_ptr()
        };
        match libc::getaddrinfo(node, serv, &hints, base.as_mut_ptr()) {
            0 => Ok(base.assume_init()),
            err => Err(ResolverError::from_raw(err)),
        }
    }
}

pub fn freeaddrinfo(ai: *mut libc::addrinfo) {
    unsafe { libc::freeaddrinfo(ai) }
}
