use crate::error::ResolverError;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr::{self, NonNull};

pub fn getaddrinfo(
    node: &CStr,
    serv: &CStr,
    hints: libc::addrinfo,
) -> Result<NonNull<libc::addrinfo>, ResolverError> {
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
    let mut base = MaybeUninit::<*mut libc::addrinfo>::uninit();
    match unsafe { libc::getaddrinfo(node, serv, &hints, base.as_mut_ptr()) } {
        0 => Ok(unsafe { NonNull::new_unchecked(base.assume_init()) }),
        err => Err(unsafe { ResolverError::from_raw(err) }),
    }
}

pub unsafe fn freeaddrinfo(ai: NonNull<libc::addrinfo>) {
    libc::freeaddrinfo(ai.as_ptr())
}
