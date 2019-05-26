//

use libc;
use socket_base::Endpoint;
use std::io;
use std::marker::PhantomData;
use std::mem;

const UNIX_MAX_PATH: usize = mem::size_of::<libc::sockaddr_un>() - 2;

pub struct LocalEndpoint<P> {
    sun: libc::sockaddr_un,
    len: u8,
    _marker: PhantomData<P>,
}

impl<P> LocalEndpoint<P> {
    pub fn new(path: &str) -> io::Result<Self> {
        use error::NAME_TOO_LONG;

        let bytes = path.as_bytes();
        if bytes.len() >= UNIX_MAX_PATH - 1 {
            return Err(NAME_TOO_LONG.into());
        }

        Ok(LocalEndpoint {
            sun: libc::sockaddr_un {
                sun_family: libc::AF_LOCAL as u16,
                sun_path: unsafe {
                    let mut sun_path: [i8; UNIX_MAX_PATH] = mem::zeroed();
                    let bytes = mem::transmute(bytes);
                    sun_path.clone_from_slice(bytes);
                    sun_path
                },
            },
            len: bytes.len() as u8,
            _marker: PhantomData,
        })
    }
}

impl<P> Endpoint<P> for LocalEndpoint<P> {
    fn as_ptr(&self) -> *const libc::sockaddr {
        &self.sun as *const _ as *const _
    }

    fn as_mut_ptr(&mut self) -> *mut libc::sockaddr {
        &mut self.sun as *mut _ as *mut _
    }

    fn capacity(&self) -> libc::socklen_t {
        mem::size_of::<libc::sockaddr_un>() as _
    }

    fn size(&self) -> libc::socklen_t {
        self.len as _
    }

    unsafe fn resize(&mut self, len: libc::socklen_t) {
        assert!(len < (UNIX_MAX_PATH - 1) as _);
        self.len = len as u8
    }
}
