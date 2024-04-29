use crate::error::OsError;
use crate::socket_base::{
    AddressFamily, Endpoint, IntoProtocolType, Protocol, SockaddrType, SocklenType,
};
use std::ffi::OsStr;
use std::fmt;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::path::{Path, PathBuf};
use std::slice;
use std::str;

const SIZE_OF_SOCKADDR_UN: SocklenType = 110;
const UNIX_MAX_PATH: usize = 108;

pub struct LocalProtocol;

impl Into<i32> for LocalProtocol {
    fn into(self) -> i32 {
        0
    }
}

impl IntoProtocolType for LocalProtocol {}

fn into_sun_path(path: &[u8], off: usize) -> Result<[i8; UNIX_MAX_PATH], OsError> {
    let mut buf: [u8; UNIX_MAX_PATH] = [0; UNIX_MAX_PATH];
    let path = path;
    if path.len() + off < buf.len() {
        buf[off..path.len()].copy_from_slice(path);
        Ok(unsafe { mem::transmute(buf) })
    } else {
        Err(OsError::NAME_TOO_LONG)
    }
}

#[derive(Debug)]
pub enum LocalAddr {
    Path(PathBuf),
    Abstract(String),
    Unnamed,
}

pub struct LocalEndpoint<P> {
    sun: libc::sockaddr_un,
    len: SocklenType,
    _marker: PhantomData<P>,
}

impl<P> LocalEndpoint<P> {
    pub fn new<T>(addr: T) -> Result<Self, OsError>
    where
        T: AsRef<LocalAddr>,
    {
        match addr.as_ref() {
            LocalAddr::Path(path) => Self::new_path(path),
            LocalAddr::Abstract(name) => Self::new_abstract(name),
            LocalAddr::Unnamed => Ok(Self::new_unnamed()),
        }
    }

    pub fn new_path<T>(path: T) -> Result<Self, OsError>
    where
        T: AsRef<Path>,
    {
        let path = path.as_ref().as_os_str().as_encoded_bytes();

        Ok(Self {
            sun: libc::sockaddr_un {
                sun_family: AddressFamily::UNIX.0,
                sun_path: into_sun_path(path, 0)?,
            },
            len: 2 + path.len() as SocklenType,
            _marker: PhantomData,
        })
    }

    pub fn new_abstract<T>(name: T) -> Result<Self, OsError>
    where
        T: AsRef<str>,
    {
        let name = name.as_ref().as_bytes();
        if name.len() > 0 {
            Ok(Self {
                sun: libc::sockaddr_un {
                    sun_family: AddressFamily::UNIX.0,
                    sun_path: into_sun_path(name, 1)?,
                },
                len: 3 + name.len() as SocklenType,
                _marker: PhantomData,
            })
        } else {
            Ok(Self::new_unnamed())
        }
    }

    pub const fn new_unnamed() -> Self {
        Self {
            sun: libc::sockaddr_un {
                sun_family: AddressFamily::UNIX.0,
                sun_path: [0; UNIX_MAX_PATH],
            },
            len: 2,
            _marker: PhantomData,
        }
    }

    fn as_path(&self) -> Option<&Path> {
        if self.len > 2 && self.sun.sun_path[2] == 0 {
            None
        } else {
            let bytes = &self.sun.sun_path[2..self.len as usize];
            unsafe {
                let bytes = slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len());
                Some(Path::new(OsStr::from_encoded_bytes_unchecked(bytes)))
            }
        }
    }

    fn as_abstract(&self) -> Option<&str> {
        if self.len > 2 && self.sun.sun_path[2] != 0 {
            None
        } else {
            let bytes = &self.sun.sun_path[3..self.len as usize];
            unsafe {
                let bytes = slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len());
                Some(str::from_utf8(bytes).unwrap())
            }
        }
    }

    pub const fn is_unnamed(&self) -> bool {
        self.len == 2
    }

    pub fn addr(&self) -> LocalAddr {
        if self.is_unnamed() {
            LocalAddr::Unnamed
        } else if let Some(path) = self.as_path() {
            LocalAddr::Path(path.into())
        } else if let Some(name) = self.as_abstract() {
            LocalAddr::Abstract(name.into())
        } else {
            unreachable!()
        }
    }
}

impl<P> Endpoint for LocalEndpoint<P>
where
    P: Protocol,
{
    const SIZE: SocklenType = SIZE_OF_SOCKADDR_UN;

    fn as_ptr(&self) -> SockaddrType {
        &self.sun as *const _ as SockaddrType
    }

    fn len(&self) -> SocklenType {
        self.len
    }

    unsafe fn init(sa: MaybeUninit<Self>, len: SocklenType) -> Self {
        if len as usize >= mem::size_of::<Self>() {
            panic!()
        }

        let mut ep: Self = sa.assume_init();
        ep.len = len;
        if ep.is_unnamed() {
            ep
        } else if let Some(_) = ep.as_path() {
            ep
        } else if let Some(_) = ep.as_abstract() {
            ep
        } else {
            panic!()
        }
    }
}

impl<P> fmt::Debug for LocalEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LocalEndpoint {{ {:?} }}", self.addr())
    }
}
