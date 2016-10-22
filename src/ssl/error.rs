use std::result;
use std::io;
use std::mem;
use std::fmt;
use std::error::Error;
use std::ffi::CStr;
use error::ErrCode;
use libc::{c_void, c_int, c_ulong};
use openssl_sys::{ERR_get_error, ERR_reason_error_string};
use super::ffi::*;

pub fn clear_error() {
    unsafe { ERR_clear_error(mem::uninitialized()) };
}

enum SslErrorImpl {
    Ssl(c_ulong),
    Sys(io::Error),
}

pub struct SslError(SslErrorImpl);

impl SslError {
    pub fn last_ssl_error() -> SslError {
        SslError(SslErrorImpl::Ssl(unsafe { ERR_get_error() }))
    }

    pub fn last_sys_error() -> SslError {
        SslError(SslErrorImpl::Sys(io::Error::from_raw_os_error(unsafe { ERR_get_error() as i32 })))
    }
}

impl Error for SslError {
    fn description(&self) -> &str {
        match &self.0 {
            &SslErrorImpl::Ssl(err) => unsafe { CStr::from_ptr(ERR_reason_error_string(err)) }.to_str().unwrap(),
            &SslErrorImpl::Sys(ref err) => err.description(),
        }
    }
}

impl fmt::Display for SslError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl fmt::Debug for SslError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<io::Error> for SslError {
    fn from(err: io::Error) -> SslError {
        SslError(SslErrorImpl::Sys(err))
    }
}

pub type SslResult<T> = result::Result<T, SslError>;
