use crate::error::OsError;
use crate::sockaddr::ffi::SockAddrUnix;
use crate::socket_base::{AddressFamily, Endpoint, IntoProtocolType, Protocol};
use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::str;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalProtocol;

impl Into<i32> for LocalProtocol {
    fn into(self) -> i32 {
        0
    }
}

impl IntoProtocolType for LocalProtocol {}

// fn into_sun_path(path: &[u8], off: usize) -> Result<[i8; UNIX_MAX_PATH], OsError> {
//     let mut buf = [0u8; UNIX_MAX_PATH];
//     let path = path;
//     if path.len() + off < buf.len() {
//         buf[off..path.len()].copy_from_slice(path);
//         Ok(unsafe { mem::transmute(buf) })
//     } else {
//         Err(OsError::NAME_TOO_LONG)
//     }
// }

#[derive(Clone, Debug)]
pub enum LocalAddr {
    Path(PathBuf),
    Abstract(String),
    Unnamed,
}

#[derive(Copy, Clone)]
pub struct LocalEndpoint<P> {
    inner: SockAddrUnix,
    _marker: PhantomData<P>,
}

impl<P> LocalEndpoint<P> {
    pub fn new<T>(addr: &LocalAddr) -> Result<Self, OsError> {
        match addr {
            LocalAddr::Path(path) => Self::new_path(path),
            LocalAddr::Abstract(name) => Self::new_abstract(name),
            LocalAddr::Unnamed => Ok(Self::new_unnamed()),
        }
    }

    pub fn new_path<T>(path: T) -> Result<Self, OsError>
    where
        T: AsRef<Path>,
    {
        Ok(Self {
            inner: SockAddrUnix::new_path(path.as_ref())?,
            _marker: PhantomData,
        })
    }

    pub fn new_abstract<T>(name: T) -> Result<Self, OsError>
    where
        T: AsRef<str>,
    {
        Ok(Self {
            inner: SockAddrUnix::new_abstract(name.as_ref())?,
            _marker: PhantomData,
        })
    }

    pub const fn new_unnamed() -> Self {
        Self {
            inner: SockAddrUnix::new_unnamed(),
            _marker: PhantomData,
        }
    }

    fn as_path(&self) -> Option<&Path> {
        self.inner.as_path()
    }

    fn as_abstract(&self) -> Option<&str> {
        self.inner.as_abstract()
    }

    pub const fn is_unnamed(&self) -> bool {
        self.inner.is_unnamed()
    }

    pub const fn family_type(&self) -> AddressFamily {
        self.inner.family_type()
    }

    pub fn addr(&self) -> LocalAddr {
        if let Some(path) = self.as_path() {
            LocalAddr::Path(path.into())
        } else if let Some(name) = self.as_abstract() {
            LocalAddr::Abstract(name.into())
        } else {
            LocalAddr::Unnamed
        }
    }

    pub const fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

impl<P> Endpoint for LocalEndpoint<P>
where
    P: Protocol,
{
    type SockAddr = SockAddrUnix;

    fn new(sa: Self::SockAddr) -> Self {
        Self {
            inner: sa,
            _marker: PhantomData,
        }
    }

    fn sockaddr(&self) -> &Self::SockAddr {
        &self.inner
    }
}

// impl<P> Endpoint for LocalEndpoint<P>
// where
//     P: Protocol,
// {
//     const MAX_SIZE: SocklenType = SIZE_OF_SOCKADDR_UN;
//
//     fn as_ptr(&self) -> SockaddrType {
//         self.inner.as_ptr()
//     }
//
//     fn len(&self) -> SocklenType {
//         self.inner.len()
//     }
//
//     unsafe fn init(ep: MaybeUninit<Self>, len: SocklenType) -> Self {
//         panic!()
//         // if len >= Self::SIZE {
//         //     panic!()
//         // }
//         //
//         // let mut ep = ep.assume_init();
//         // ep.len = len;
//         // if ep.is_unnamed() {
//         //     ep
//         // } else if let Some(_) = ep.as_path() {
//         //     ep
//         // } else if let Some(_) = ep.as_abstract() {
//         //     ep
//         // } else {
//         //     panic!()
//         // }
//     }
// }

impl<P> fmt::Debug for LocalEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LocalEndpoint {{ {:?} }}", self.addr())
    }
}
