use ffi::sockaddr;
use std::mem;

#[derive(Clone)]
pub struct BsdSockAddr<T> {
    pub sa: T,
}

impl<T: super::PodTrait> BsdSockAddr<T> {
    pub fn new(sa_family: i32, sa_len: u8) -> BsdSockAddr<T> {
        let mut sai: Self = BsdSockAddr { sa: unsafe { mem::uninitialized() } };
        let sa = unsafe { &mut *(&mut sai.sa as *mut _ as *mut sockaddr) };
        sa.sa_len = sa_len;
        sa.sa_family = sa_family as u8;
        sai
    }

    pub fn from(sa: *const T, sa_len: u8) -> BsdSockAddr<T> {
        let mut sai = BsdSockAddr { sa: unsafe { mem::transmute_copy(&*sa) } };
        let sa = unsafe { &mut *(&mut sai.sa as *mut _ as *mut sockaddr) };
        sa.sa_len = sa_len;
        sai
    }

    pub fn capacity(&self) -> usize {
        mem::size_of_val(&self.sa)
    }

    pub fn size(&self) -> u8 {
        unsafe { &*(&self.sa as *const _ as *const sockaddr) }.sa_len
    }

    pub fn resize(&mut self, sa_len: u8) {
        unsafe { &mut *(&mut self.sa as *mut _ as *mut sockaddr) }.sa_len = sa_len;
    }
}

impl BsdSockAddr<Box<[u8]>> {
    pub fn from_vec(vec: Vec<u8>, sa_len: u8) -> BsdSockAddr<Box<[u8]>> {
        let mut sa = vec.into_boxed_slice();
        sa[0] = sa_len;
        BsdSockAddr { sa: sa }
    }

    pub fn capacity(&self) -> usize {
        self.sa.len()
    }

    pub fn size(&self) -> u8 {
        self.sa[0] as u8
    }

    pub fn resize(&mut self, sa_len: u8) {
        self.sa[0] = sa_len
    }
}
