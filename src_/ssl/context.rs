use ssl::*;
use ssl::ffi::*;

use std::mem;
use std::ptr;
use std::slice;
use std::path::Path;
use std::ffi::{CString, CStr};
use std::sync::{Arc};
use libc::{c_void, c_char, c_int, c_long};
use openssl_sys::*;
use openssl::types::OpenSslTypeRef;

struct BioBox { handle: *mut BIO }

impl BioBox {
    fn new_mem_buf(buf: &[u8]) -> Result<BioBox> {
        let handle = unsafe { BIO_new_mem_buf(buf.as_ptr() as *mut c_void, buf.len() as c_int) };
        if handle.is_null() {
            Err(Error::last_ssl_error())
        } else {
            Ok(BioBox { handle: handle })
        }
    }

    fn new_file(path: &CStr) -> Result<BioBox> {
        use libc::fopen;

        let fp = unsafe { fopen(path.as_ptr(), b"r".as_ptr() as *const i8) };
        if fp.is_null() {
            // NOTE: BIO_new_file が openssl-sys に定義されていないので、
            //       BIO_new_fp で代用したことにより、エラーコードが正しくないかもしれない
            return Err(Error::last_ssl_error());
        }

        let handle = unsafe { BIO_new_fp(fp, 1) }; // 1 is close_flag
        if handle.is_null() {
            return Err(Error::last_ssl_error());
        } else {
            Ok(BioBox { handle: handle })
        }
    }
}

impl Drop for BioBox {
    fn drop(&mut self) {
        unsafe  { BIO_free_all(self.handle) }
    }
}

struct X509Box { handle: *mut X509 }

impl X509Box {
    fn pem_read_bio(bio: *mut BIO) -> Result<X509Box> {
        let handle = unsafe { PEM_read_bio_X509(bio, ptr::null_mut(), None, ptr::null_mut()) };
        if handle.is_null() {
            Err(Error::last_ssl_error())
        } else {
            Ok(X509Box { handle: handle })
        }
    }
}

impl Drop for X509Box {
    fn drop(&mut self) {
        unsafe { X509_free(self.handle) }
    }
}

struct DHBox { handle: *mut DH }

impl DHBox {
    fn pem_read_bio(bio: *mut BIO) -> Result<DHBox> {
        let handle = unsafe { PEM_read_bio_DHparams(bio, ptr::null_mut(), None, ptr::null_mut()) };
        if handle.is_null() {
            Err(Error::last_ssl_error())
        } else {
            Ok(DHBox { handle: handle })
        }
    }
}

impl Drop for DHBox {
    fn drop(&mut self) {
        unsafe { DH_free(self.handle) }
    }
}

struct SslContextImpl {
    handle: *mut SSL_CTX,
    verify_callback: Box<Fn(bool, &SslVerifyContext) -> bool>,
    passwd_callback: Box<Fn(&mut [u8], PasswordPurpose) -> usize>,
}

impl SslContextImpl {
    fn new(method: unsafe extern "C" fn() -> *const SSL_METHOD) -> SslContextImpl {
        init();

        let handle = unsafe { SSL_CTX_new(method()) };
        assert!( !handle.is_null() );
        SslContextImpl {
            handle: handle,
            verify_callback: Box::new(|_, _| false),
            passwd_callback: Box::new(|_, _| 0),
        }
    }

    unsafe extern "C" fn password_callback(buf: *mut c_char, size: c_int, rwflag: c_int, user_data: *mut c_void) -> c_int {
        let ctx = &*(user_data as *const SslContextImpl);
        let buf = slice::from_raw_parts_mut(buf as *mut u8, size as usize);
        (*ctx.passwd_callback)(buf, mem::transmute(rwflag)) as c_int
    }

    fn set_password_callback<F>(&mut self, callback: F) -> Result<()>
        where F: Fn(&mut [u8], PasswordPurpose) -> usize + 'static
    {
        self.passwd_callback = Box::new(callback);
        unsafe {
            SSL_CTX_set_default_passwd_cb_userdata(self.handle, self as *mut _ as *mut c_void);
            SSL_CTX_set_default_passwd_cb(self.handle, Self::password_callback);
        }
        Ok(())
    }

    extern "C" fn verify_callback(preverified: c_int, ctx: *mut X509_STORE_CTX) -> c_int {
        assert!( !ctx.is_null() );
        unsafe {
            let ssl =  X509_STORE_CTX_get_ex_data(ctx, SSL_get_ex_data_X509_STORE_CTX_idx()) as *mut SSL;
            if !ssl.is_null() {
                let this = &*(SSL_CTX_get_app_data(SSL_get_SSL_CTX(ssl)) as *mut SslContextImpl);
                return (*this.verify_callback)(preverified != 0, SslVerifyContext::from_ptr(ctx)) as i32
            }
        }
        0
    }

    fn set_verify_callback<F>(&mut self, callback: F) -> Result<()>
        where F: Fn(bool, &SslVerifyContext) -> bool + 'static
    {
        self.verify_callback = Box::new(callback);
        unsafe {
            SSL_CTX_set_app_data(self.handle, self as *mut _ as *mut c_void);
            SSL_CTX_set_verify(self.handle, SSL_CTX_get_verify_mode(self.handle), Some(Self::verify_callback));
        }
        Ok(())
    }
}

unsafe impl Send for SslContextImpl {
}

unsafe impl Sync for SslContextImpl {
}

#[derive(Clone)]
pub struct SslContext(Arc<SslContextImpl>);

impl SslContext {
    pub fn sslv23() -> SslContext {
        SslContext(Arc::new(SslContextImpl::new(SSLv23_method)))
    }

    pub fn sslv3() -> SslContext {
        SslContext(Arc::new(SslContextImpl::new(SSLv3_method)))
    }

    pub fn tlsv1() -> SslContext {
        SslContext(Arc::new(SslContextImpl::new(TLSv1_method)))
    }

    pub unsafe fn as_ptr(&self) -> *mut SSL_CTX {
        self.0.handle
    }

    pub fn add_certificate_authority() {
        // TODO
    }

    pub fn add_verify_path<P>(&self, path: P) -> Result<()>
        where P: AsRef<Path>
    {
        clear_error();
        let path = CString::new(path.as_ref().to_str().unwrap()).unwrap();
        if unsafe { SSL_CTX_load_verify_locations(self.0.handle, ptr::null(), path.as_ptr() as *const i8) } == 1 {
            Ok(())
        } else {
            Err(Error::last_ssl_error())
        }
    }

    pub fn clear_options(&self, options: SslOptions) -> Result<()> {
        unsafe { SSL_CTX_clear_options(self.0.handle, options.bits() as c_long) };
        Ok(())
    }

    pub fn load_verify_file<P>(&self, path: P) -> Result<()>
        where P: AsRef<Path>
    {
        clear_error();
        let path = CString::new(path.as_ref().to_str().unwrap()).unwrap();
        if unsafe { SSL_CTX_load_verify_locations(self.0.handle, path.as_ptr() as *const i8, ptr::null()) } == 1 {
            Ok(())
        } else {
            Err(Error::last_ssl_error())
        }
    }

    pub fn set_default_verify_paths(&self) -> Result<()> {
        clear_error();
        if unsafe { SSL_CTX_set_default_verify_paths(self.0.handle) } == 1 {
            Ok(())
        } else {
            Err(Error::last_ssl_error())
        }
    }

    pub fn set_options(&self, options: SslOptions) -> Result<()> {
        unsafe { SSL_CTX_set_options(self.0.handle, options.bits() as c_long) };
        Ok(())
    }

    pub fn set_password_callback<F>(&mut self, callback: F) -> Result<()>
        where F: Fn(&mut [u8], PasswordPurpose) -> usize + 'static
    {
        let ctx = &*self.0 as *const SslContextImpl;
        if let Some(imp) = Arc::get_mut(&mut self.0) {
            imp.set_password_callback(callback)
        } else {
            panic!("It has any strong reference.");  // TODO: returns a Err<Any>
        }
    }

    pub fn set_verify_callback<F>(&mut self, callback: F) -> Result<()>
        where F: Fn(bool, &SslVerifyContext) -> bool + 'static
    {
        let ctx = &*self.0 as *const SslContextImpl;
        if let Some(imp) = Arc::get_mut(&mut self.0) {
            imp.set_verify_callback(callback)
        } else {
            panic!("It has any strong reference.");  // TODO: returns a Err<Any>
        }
    }

    pub fn set_verify_depth(&self, depth: i32) -> Result<()> {
        unsafe { SSL_CTX_set_verify_depth(self.0.handle, depth) };
        Ok(())
    }

    pub fn set_verify_mode(&self, mode: SslVerifyMode) -> Result<()> {
        unsafe { SSL_CTX_set_verify(self.0.handle, mode.bits(), SSL_CTX_get_verify_callback(self.0.handle)) };
        Ok(())
    }

    pub fn use_certificate(&self, cert: &[u8], fmt: FileFormat) -> Result<()> {
        clear_error();
        match fmt {
            FileFormat::PEM => {
                let bio = try!(BioBox::new_mem_buf(cert));
                let cert = try!(X509Box::pem_read_bio(bio.handle));
                if unsafe { SSL_CTX_use_certificate(self.0.handle, cert.handle) } == 1 {
                    return Ok(());
                }
            },
        }
        Err(Error::last_ssl_error())
    }

    pub fn use_certificate_chain() {
        // TODO
    }

    pub fn use_certificate_chain_file<P>(&self, path: P) -> Result<()>
        where P: AsRef<Path>
    {
        clear_error();
        let path = CString::new(path.as_ref().to_str().unwrap()).unwrap();
        if unsafe { SSL_CTX_use_certificate_chain_file(self.0.handle, path.as_ptr() as *const i8) } == 1 {
            Ok(())
        } else {
            Err(Error::last_ssl_error())
        }
    }

    pub fn use_certificate_file<P>(&self, path: P, fmt: FileFormat) -> Result<()>
        where P: AsRef<Path>
    {
        clear_error();
        let path = CString::new(path.as_ref().to_str().unwrap()).unwrap();
        if unsafe { SSL_CTX_use_certificate_file(self.0.handle, path.as_ptr() as *const i8, fmt as i32) } == 1 {
            Ok(())
        } else {
            Err(Error::last_ssl_error())
        }
    }

    pub fn use_private_key() {
        // TOOD
    }

    pub fn use_private_key_file<P>(&self, path: P, fmt: FileFormat) -> Result<()>
        where P: AsRef<Path>
    {
        clear_error();
        match fmt {
            FileFormat::PEM => {
                let path = CString::new(path.as_ref().to_str().unwrap()).unwrap();
                if unsafe { SSL_CTX_use_PrivateKey_file(self.0.handle, path.as_ptr() as *const i8, fmt as i32) } == 1 {
                    Ok(())
                } else {
                    Err(Error::last_ssl_error())
                }
            },
        }
    }

    pub fn use_rsa_private_key() {
        // TODO
    }

    pub fn use_rsa_prive_key_file() {
        // TODO
    }

    fn do_use_tmp_dh(&self, bio: BioBox) -> Result<()> {
        let dh = try!(DHBox::pem_read_bio(bio.handle));
        if unsafe { SSL_CTX_set_tmp_dh(self.0.handle, dh.handle) } == 1 {
            Ok(())
        } else {
            Err(Error::last_ssl_error())
        }
    }

    pub fn use_tmp_dh(&self, dh: &[u8]) -> Result<()> {
        clear_error();
        let bio = try!(BioBox::new_mem_buf(dh));
        self.do_use_tmp_dh(bio)
    }

    pub fn use_tmp_dh_file<P>(&self, path: P) -> Result<()>
        where P: AsRef<Path>
    {
        clear_error();
        let path = CString::new(path.as_ref().to_str().unwrap()).unwrap();
        let bio = try!(BioBox::new_file(&path));
        self.do_use_tmp_dh(bio)
    }
}

#[test]
fn test_sslv23() {
    let _ = SslContext::sslv23();
}

#[test]
fn test_sslv3() {
    let _ = SslContext::sslv3();
}

#[test]
fn test_tlsv1() {
    let _ = SslContext::tlsv1();
}
