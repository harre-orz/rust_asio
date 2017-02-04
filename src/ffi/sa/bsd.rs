use super::PODTrait;
use ffi::{sockaddr, sockaddr_in, sockaddr_in6, sockaddr_storage, sockaddr_un};

use std::mem;

impl PODTrait for sockaddr_in { }
impl PODTrait for sockaddr_in6 { }
impl PODTrait for sockaddr_storage { }
impl PODTrait for sockaddr_un { }

#[derive(Clone)]
pub struct BSDSockAddrImpl<T> {
    sa: T,
}

impl<T: PODTrait> BSDSockAddrImpl<T> {
    pub fn new(sa_family: i32, sa_len: usize) -> BSDSockAddrImpl<T> {
        let mut sai: Self = BSDSockAddrImpl {
            sa: unsafe { mem::uninitialized() }
        };
        sai.as_mut_sa().sa_len = sa_len as u8;
        sai.as_mut_sa().sa_family = sa_family as u8;
        sai
    }

    pub fn capacity(&self) -> usize {
        mem::size_of_val(&self.sa)
    }

    pub fn data(&self) -> *mut () {
        &self.sa as *const _ as *mut _
    }

    pub fn size(&self) -> usize {
        self.as_sa().sa_len as usize
    }

    pub fn resize(&mut self, sa_len: usize) {
        self.as_mut_sa().sa_len = sa_len as u8
    }

    fn as_sa(&self) -> &sockaddr {
        unsafe { &*(self.data() as *const sockaddr) }
    }

    fn as_mut_sa(&mut self) -> &mut sockaddr {
        unsafe { &mut *(self.data() as *mut sockaddr) }
    }
}

impl BSDSockAddrImpl<Box<[u8]>> {
    pub fn from_vec(vec: Vec<u8>, sa_len: usize) -> BSDSockAddrImpl<Box<[u8]>> {
        let mut sa = vec.into_boxed_slice();
        sa[0] = sa_len as u8;
        BSDSockAddrImpl {
            sa: sa,
        }
    }

    pub fn capacity(&self) -> usize {
        self.sa.len()
    }

    pub fn data(&self) -> *mut () {
        self.sa.as_ptr() as *mut _
    }

    pub fn size(&self) -> usize {
        self.sa[0] as usize
    }

    pub fn resize(&mut self, sa_len: usize) {
        self.sa[0] = sa_len as u8
    }
}
