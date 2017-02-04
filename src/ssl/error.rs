use std::error;
use std::result;
use std::mem;
use std::str;
use std::fmt;
use std::io;
use openssl;
use openssl_sys::*;

#[derive(Debug)]
pub enum Error {
    Ssl(openssl::error::Error),
    Sys(io::Error),
}


impl Error {
    pub fn last_ssl_error() -> Error {
        Error::Ssl(unsafe { mem::transmute(ERR_get_error()) })
    }

    pub fn last_sys_error() -> Error {
        Error::Sys(io::Error::from_raw_os_error(unsafe { ERR_get_error() as i32 }))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::Ssl(ref err) => write!(f, "{}", err),
            &Error::Sys(ref err) => write!(f, "{}", err),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Sys(err)
    }
}

impl From<openssl::error::Error> for Error {
    fn from(err: openssl::error::Error) -> Error {
        Error::Ssl(err)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            &Error::Ssl(ref err) => err.description(),
            &Error::Sys(ref err) => err.description(),
        }
    }
}

pub type Result<T> = result::Result<T, Error>;
