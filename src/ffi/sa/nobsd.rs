use super::PODTrait;
use ffi::{self, sockaddr};

use std::mem;

impl PODTrait for ffi::sockaddr_in { }
impl PODTrait for ffi::sockaddr_in6 { }
impl PODTrait for ffi::sockaddr_storage { }

#[cfg(unix)]
impl PODTrait for ffi::sockaddr_un { }

#[derive(Clone)]
pub struct SockAddrImpl<T> {
    sa: T,
    sa_len: u8,
}

impl<T> SockAddrImpl<T> {
    pub fn size(&self) -> usize {
        self.sa_len as usize
    }

    pub fn resize(&mut self, sa_len: usize) {
        self.sa_len = sa_len as _
    }
}

impl<T: PODTrait> SockAddrImpl<T> {
    pub fn new(sa_family: i32, sa_len: usize) -> SockAddrImpl<T> {
        let mut sai = SockAddrImpl {
            sa: unsafe { mem::uninitialized() },
            sa_len: sa_len as _
        };
        let sa: &mut sockaddr = unsafe { &mut *(&mut sai as *mut _ as *mut _) };
        sa.sa_family = sa_family as u16;
        sai
    }

    pub fn capacity(&self) -> usize {
        mem::size_of_val(&self.sa)
    }

    pub fn data(&self) -> *const () {
        &self.sa as *const _ as *const _
    }
}

impl SockAddrImpl<Box<[u8]>> {
    pub fn from_vec(sa: Vec<u8>, sa_len: usize) -> SockAddrImpl<Box<[u8]>> {
        SockAddrImpl {
            sa: sa.into_boxed_slice(),
            sa_len: sa_len as _,
        }
    }

    pub fn capacity(&self) -> usize {
        self.sa.len()
    }

    pub fn data(&self) -> *const () {
        self.sa.as_ptr() as *const _
    }
}
