use std::ptr;
use std::cmp;
use std::sync::Mutex;
use libc::{c_void, c_int, size_t};
use error::eof;
use streambuf::StreamBuf;
use super::*;
use super::ffi::*;
use openssl_sys::*;

lazy_static! {
    static ref ACCEPT_MUTEX: Mutex<()> = Mutex::new(());
}

fn do_accept(ssl: *mut SSL, _: *mut c_void, _: size_t) -> c_int {
    let _lock = ACCEPT_MUTEX.lock();
    unsafe { SSL_accept(ssl) }
}

fn do_connect(ssl: *mut SSL, _: *mut c_void, _: size_t) -> c_int {
    unsafe { SSL_connect(ssl) }
}

fn do_read(ssl: *mut SSL, data: *mut c_void, len: size_t) -> c_int {
    assert!(len <= i32::max_value() as size_t);
    unsafe { SSL_read(ssl, data, len as c_int) }
}

fn do_write(ssl: *mut SSL, data: *mut c_void, len: size_t) -> c_int {
    assert!(len <= i32::max_value() as size_t);
    unsafe { SSL_write(ssl, data, len as c_int) }
}

fn do_shutdown(ssl: *mut SSL, _: *mut c_void, _: size_t) -> c_int {
    let res = unsafe { SSL_shutdown(ssl) };
    if res == 0 {
        unsafe { SSL_shutdown(ssl) }
    } else {
        res
    }
}

pub enum Want {
    InputAndRetry = -2,

    OutputAndRetry = -1,

    Nothing = 0,

    Output = 1,
}

pub struct Engine {
    ssl: *mut SSL,
    ext_bio: *mut BIO,
    verify_callback: Box<Fn(bool, SslVerifyContext) -> bool>,
}

impl Engine {
    pub fn new(ctx: &SslContext) -> SslResult<Engine> {
        let ssl = unsafe { SSL_new(ctx.raw_handle()) };
        if ssl.is_null() {
            return Err(SslError::last_ssl_error());
        }

        unsafe {
            SSL_set_mode(ssl, SSL_MODE_ENABLE_PARTIAL_WRITE);
            SSL_set_mode(ssl, SSL_MODE_ACCEPT_MOVING_WRITE_BUFFER);
            SSL_set_mode(ssl, SSL_MODE_RELEASE_BUFFERS);
        }

        let mut ext_bio = ptr::null_mut();
        let mut int_bio = ptr::null_mut();
        unsafe {
            BIO_new_bio_pair(&mut int_bio, 4096, &mut ext_bio, 4096);
            SSL_set_bio(ssl, int_bio, int_bio);
        }
        Ok(Engine {
            ssl: ssl,
            ext_bio: ext_bio,
            verify_callback: Box::new(|_,_| false),
        })
    }

    pub fn set_verify_mode(&self, mode: VerifyMode) -> SslResult<()> {
        unsafe { SSL_set_verify(self.ssl, mode as i32, SSL_get_verify_callback(self.ssl)) };
        Ok(())
    }

    pub fn set_verify_depth(&self, depth: i32) -> SslResult<()> {
        unsafe { SSL_set_verify_depth(self.ssl, depth as c_int) };
        Ok(())
    }

    extern "C" fn verify_callback(preverified: c_int, ctx: *mut X509_STORE_CTX) -> c_int {
        if !ctx.is_null() {
            let ssl = unsafe { X509_STORE_CTX_get_ex_data(ctx, SSL_get_ex_data_X509_STORE_CTX_idx()) as *mut SSL };
            if !ssl.is_null() {
                let this = unsafe { &*(SSL_get_app_data(ssl) as *const Self) };
                return (*this.verify_callback)(preverified != 0, SslVerifyContext(ctx)) as c_int;
            }
        }
        0
    }

    pub fn set_verify_callback<F>(&mut self, callback: F) -> SslResult<()>
        where F: Fn(bool, SslVerifyContext) -> bool + 'static
    {
        let user_data = self as *mut Self;
        self.verify_callback = Box::new(callback);
        unsafe {
            SSL_set_app_data(self.ssl, user_data as *mut c_void);
            SSL_set_verify(self.ssl, SSL_get_verify_mode(self.ssl), Some(Self::verify_callback));
        }
        Ok(())
    }

    fn perform(&self, op: fn(*mut SSL, *mut c_void, size_t) -> c_int, data: *mut c_void, len: size_t) -> (Want, SslResult<usize>) {
        let pending_output_before = unsafe { BIO_ctrl_pending(self.ext_bio) };
        clear_error();
        let res = op(self.ssl, data, len);
        let err = unsafe { SSL_get_error(self.ssl, res) };
        let pending_output_after = unsafe { BIO_ctrl_pending(self.ext_bio) };

        if err == SSL_ERROR_SSL {
            (Want::Nothing, Err(SslError::last_ssl_error()))
        }
        else if err == SSL_ERROR_SYSCALL {
            (Want::Nothing, Err(SslError::last_sys_error()))
        }
        else if err == SSL_ERROR_WANT_WRITE {
            (Want::OutputAndRetry, Ok(res as usize))
        }
        else if pending_output_after > pending_output_before {
            (if res > 0 { Want::Output } else { Want::OutputAndRetry }, Ok(res as usize))
        }
        else if err == SSL_ERROR_WANT_READ {
            (Want::InputAndRetry, Ok(res as usize))
        }
        else if unsafe { SSL_get_shutdown(self.ssl) } & SSL_RECEIVED_SHUTDOWN != 0 {
            (Want::Nothing, Err(eof().into()))
        }
        else {
            (Want::Nothing, Ok(res as usize))
        }
    }

    pub fn handshake(&self, mode: Handshake) -> (Want, SslResult<usize>) {
        self.perform(match mode {
            Handshake::Client => do_connect,
            Handshake::Server => do_accept,
        }, ptr::null_mut(), 0)
    }

    pub fn shutdown(&self) -> (Want, SslResult<usize>) {
        self.perform(do_shutdown, ptr::null_mut(), 0)
    }

    pub fn read(&self, buf: &mut [u8]) -> (Want, SslResult<usize>) {
        if buf.len() == 0 {
            (Want::Nothing, Ok(0))
        } else {
            self.perform(do_read, buf.as_ptr() as *mut c_void, buf.len())
        }
    }

    pub fn write(&self, buf: &[u8]) -> (Want, SslResult<usize>) {
        if buf.len() == 0 {
            (Want::Nothing, Ok(0))
        } else {
            self.perform(do_write, buf.as_ptr() as *const _ as *mut c_void, buf.len())
        }
    }

    pub fn get_output(&self, sbuf: &mut StreamBuf) {
        let len = {
            let buf = sbuf.prepare(4096).unwrap();
            unsafe { BIO_read(self.ext_bio, buf.as_ptr() as *mut c_void, buf.len() as c_int) }
        };
        sbuf.commit(len as usize);
    }

    pub fn put_input(&self, sbuf: &mut StreamBuf) {
        let len = {
            let buf = sbuf.as_slice();
            unsafe { BIO_write(self.ext_bio, buf.as_ptr() as *const c_void, buf.len() as c_int) }
        };
        sbuf.consume(len as usize);
    }

    pub fn map_error_code(&self) {
        unsafe { BIO_wpending(self.ext_bio) };
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            BIO_free_all(self.ext_bio);
            SSL_free(self.ssl);
        }
    }
}
