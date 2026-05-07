use crate::error::OsError;
use crate::sockaddr::{SockAddrUnix, SockAddrWithLen, SockLen};
use crate::socket_base::{Endpoint, EndpointIter, Endpoints, Protocol};
use std::ffi::OsStr;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::result;
use std::{fmt, slice};

type Result<T> = result::Result<T, OsError>;

#[derive(Eq, PartialEq, Debug)]
pub enum LocalAddrRef<'a> {
    Path(&'a Path),
    Abstract(&'a OsStr),
}

impl<'a> PartialEq<Path> for LocalAddrRef<'a> {
    fn eq(&self, other: &Path) -> bool {
        match self {
            &Self::Path(path) => path == other,
            _ => false,
        }
    }
}

impl<'a> PartialEq<OsStr> for LocalAddrRef<'a> {
    fn eq(&self, other: &OsStr) -> bool {
        match self {
            &Self::Abstract(name) => name == other,
            _ => false,
        }
    }
}

pub trait AsLocalAddr<'a> {
    fn as_local_addr(&self) -> LocalAddrRef<'a>;
}

impl<'a> AsLocalAddr<'a> for LocalAddrRef<'a> {
    fn as_local_addr(&self) -> Self {
        match self {
            LocalAddrRef::Path(path) => LocalAddrRef::Path(path),
            LocalAddrRef::Abstract(name) => LocalAddrRef::Abstract(name),
        }
    }
}

impl<'a> AsLocalAddr<'a> for &'a LocalAddrRef<'a> {
    fn as_local_addr(&self) -> LocalAddrRef<'a> {
        match self {
            LocalAddrRef::Path(path) => LocalAddrRef::Path(path),
            LocalAddrRef::Abstract(name) => LocalAddrRef::Abstract(name),
        }
    }
}

impl<'a> AsLocalAddr<'a> for &'a Path {
    fn as_local_addr(&self) -> LocalAddrRef<'a> {
        LocalAddrRef::Path(self)
    }
}

impl<'a> AsLocalAddr<'a> for &'a PathBuf {
    fn as_local_addr(&self) -> LocalAddrRef<'a> {
        LocalAddrRef::Path(self)
    }
}

impl<'a> AsLocalAddr<'a> for &'a OsStr {
    fn as_local_addr(&self) -> LocalAddrRef<'a> {
        LocalAddrRef::Abstract(self)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[non_exhaustive]
pub struct LocalProtocol;

impl Into<i32> for LocalProtocol {
    fn into(self) -> i32 {
        0
    }
}

#[derive(Copy, Clone)]
pub struct LocalEndpoint<P> {
    sun: SockAddrUnix,
    #[cfg(not(target_os = "macos"))]
    sun_len: SockLen,
    _marker: PhantomData<P>,
}

impl<P> LocalEndpoint<P> {
    pub fn new<'a, T>(local_addr: T) -> Result<Self>
    where
        T: AsLocalAddr<'a>,
    {
        let sun = match local_addr.as_local_addr() {
            LocalAddrRef::Path(path) => {
                SockAddrUnix::new(path.as_os_str().as_encoded_bytes(), false)
            }
            LocalAddrRef::Abstract(name) => SockAddrUnix::new(name.as_encoded_bytes(), true),
        };
        let (sun, sun_len) = sun?.unwrap();
        Ok(Self {
            sun: sun,
            #[cfg(not(target_os = "macos"))]
            sun_len: sun_len,
            _marker: PhantomData,
        })
    }

    #[cfg(not(target_os = "macos"))]
    pub const fn len(&self) -> SockLen {
        self.sun_len
    }
    #[cfg(target_os = "macos")]
    pub const fn len(&self) -> SockLen {
        self.sun.len() as SockLen
    }

    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { self.sun.as_bytes_unchecked(self.len()) }
    }

    pub fn as_local_addr(&self) -> LocalAddrRef<'_> {
        let bytes = self.as_bytes();
        if bytes[2] != 0 {
            let bytes = &bytes[2..];
            unsafe {
                let bytes = slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len());
                LocalAddrRef::Path(Path::new(OsStr::from_encoded_bytes_unchecked(bytes)))
            }
        } else {
            let bytes = &bytes[3..];
            unsafe {
                let bytes = slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len());
                LocalAddrRef::Abstract(OsStr::from_encoded_bytes_unchecked(bytes))
            }
        }
    }
}

impl<P> fmt::Debug for LocalEndpoint<P>
where
    P: Protocol,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_local_addr() {
            LocalAddrRef::Path(path) => write!(f, "LocalEndpoint {{ Path({:?}) }}", path),
            LocalAddrRef::Abstract(name) => write!(f, "LocalEndpoint {{ Abstract({:?}) }}", name),
        }
    }
}

impl<P> Endpoint for LocalEndpoint<P>
where
    P: Protocol,
{
    type SockAddr = SockAddrUnix;

    fn sockaddr_ref(&self) -> &Self::SockAddr {
        &self.sun
    }

    fn sockaddr_len(&self) -> SockLen {
        self.len()
    }

    unsafe fn from_sockaddr(sa_with_len: SockAddrWithLen<Self::SockAddr>) -> Self {
        let (sun, sun_len) = sa_with_len.unwrap();
        Self {
            sun: sun,
            #[cfg(not(target_os = "macos"))]
            sun_len: sun_len,
            _marker: PhantomData,
        }
    }
}

impl<'a, P> Endpoints<'a, P> for LocalEndpoint<P>
where
    P: Protocol<Endpoint = Self> + 'a,
{
    type Iter = EndpointIter<'a, P>;

    fn endpoints(&'a self) -> Self::Iter {
        EndpointIter::new(self)
    }
}
