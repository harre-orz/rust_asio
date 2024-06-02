use crate::socket_base::{Endpoint, SockaddrType, SocklenType};
use libc;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::slice;

#[derive(Copy, Clone)]
pub struct GenericEndpoint<P> {
    ss: libc::sockaddr_storage,
    len: SocklenType,
    _marker: PhantomData<P>,
}

impl<P> GenericEndpoint<P> {
    pub fn new(bytes: &[u8]) -> Option<Self> {
        if bytes.len() >= Self::SIZE as usize {
            return None;
        }
        let mut ep = MaybeUninit::<Self>::uninit();
        {
            let ss = ep.as_mut_ptr() as *mut u8;
            let ss = unsafe { slice::from_raw_parts_mut(ss, bytes.len()) };
            ss.clone_from_slice(bytes);
        }
        let mut ep = unsafe { ep.assume_init() };
        ep.len = bytes.len() as SocklenType;
        Some(ep)
    }

    pub fn as_bytes(&self) -> &[u8] {
        let ss = &self.ss as *const _ as *const u8;
        unsafe { slice::from_raw_parts(ss, self.len as usize) }
    }
}

impl<P> Endpoint for GenericEndpoint<P> {
    const SIZE: SocklenType = mem::size_of::<libc::sockaddr_storage>() as SocklenType;

    fn as_ptr(&self) -> SockaddrType {
        &self.ss as *const _ as SockaddrType
    }

    fn len(&self) -> SocklenType {
        self.len
    }

    unsafe fn init(ep: MaybeUninit<Self>, len: SocklenType) -> Self {
        if len >= Self::SIZE {
            panic!()
        }

        let mut ep = ep.assume_init();
        ep.len = len;
        ep
    }
}
