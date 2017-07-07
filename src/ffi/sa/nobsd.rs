use ffi::sockaddr;

use std::mem;

#[derive(Clone)]
pub struct SockAddr<T> {
    sa: T,
    sa_len: u8,
}

impl<T> SockAddr<T> {
    pub fn size(&self) -> u8 {
        self.sa_len
    }

    pub fn resize(&mut self, sa_len: u8) {
        self.sa_len = sa_len
    }
}

impl<T: PODTrait> SockAddr<T> {
    pub fn new(sa_family: i32, sa_len: u8) -> SockAddr<T> {
        let mut sai = SockAddr {
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
}

impl SockAddr<Box<[u8]>> {
    pub fn from_vec(sa: Vec<u8>, sa_len: u8) -> SockAddr<Box<[u8]>> {
        SockAddr {
            sa: sa.into_boxed_slice(),
            sa_len: sa_len
        }
    }

    pub fn capacity(&self) -> usize {
        self.sa.len()
    }
}
