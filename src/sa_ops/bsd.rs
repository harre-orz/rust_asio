use std::mem;
use std::ops::{Deref, DerefMut};
use libc::sockaddr;
use super::SockAddrTrait;

#[derive(Clone)]
pub struct SockAddrImpl<T> {
    sa: T,
}

impl<T: SockAddrTrait> SockAddrImpl<T> {
    pub fn new(sa_family: i32, sa_len: usize) -> SockAddrImpl<T> {
        let mut sa: T = unsafe { mem::uninitialized() };
        {
            let sa = unsafe { &mut *(&mut sa as *mut _ as *mut sockaddr) };
            sa.sa_len = sa_len as u8;
            sa.sa_family = sa_family as u8;
        }
        SockAddrImpl {
            sa: sa,
        }
    }

    pub unsafe fn as_sockaddr<U>(&self) -> &U {
        &*(&self.sa as *const _ as *const U)
    }

    pub unsafe fn as_mut_sockaddr<U>(&mut self) -> &mut U {
        &mut *(&mut self.sa as *mut _ as *mut U)
    }

    pub fn capacity(&self) -> usize {
        mem::size_of::<T>()
    }

    pub fn size(&self) -> usize {
        let sa: &sockaddr = unsafe { self.as_sockaddr() };
        sa.sa_len as usize
    }

    pub unsafe fn resize(&mut self, size: usize) {
        let sa: &mut sockaddr = self.as_mut_sockaddr();
        sa.sa_len = size as u8;
    }
}

impl SockAddrImpl<Box<[u8]>> {
    pub fn from_vec(vec: Vec<u8>, sa_len: usize) -> SockAddrImpl<Box<[u8]>> {
        let mut sa = vec.into_boxed_slice();
        sa[0] = sa_len as u8;
        SockAddrImpl {
            sa: sa,
        }
    }

    pub unsafe fn as_sockaddr<U>(&self) -> &U {
        &*(self.sa.as_ptr() as *const U)
    }

    pub unsafe fn as_mut_sockaddr<U>(&mut self) -> &mut U {
        &mut *(self.sa.as_mut_ptr() as *mut U)
    }

    pub fn capacity(&self) -> usize {
        self.sa.len()
    }

    pub fn size(&self) -> usize {
        let sa: &sockaddr = unsafe { self.as_sockaddr() };
        sa.sa_len as usize
    }

    pub unsafe fn resize(&mut self, size: usize) {
        let sa: &mut sockaddr = self.as_mut_sockaddr();
        sa.sa_len = size as u8;
    }
}

impl<T: SockAddrTrait> Deref for SockAddrImpl<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.sa
    }
}

impl<T: SockAddrTrait> DerefMut for SockAddrImpl<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.sa
    }
}

impl Deref for SockAddrImpl<Box<[u8]>> {
    type Target = sockaddr;

    fn deref(&self) -> &sockaddr {
        unsafe { self.as_sockaddr() }
    }
}

impl DerefMut for SockAddrImpl<Box<[u8]>> {
    fn deref_mut(&mut self) -> &mut sockaddr {
        unsafe { self.as_mut_sockaddr() }
    }
}
