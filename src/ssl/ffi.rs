use openssl_sys::*;
// use libc::{c_void, c_char, c_int, c_long, size_t};
//
// pub type VerifyCallback = extern "C" fn(c_int, *mut X509_STORE_CTX) -> c_int;
//
// pub const SSL_SENT_SHUTDOWN: c_int = 1;
// pub const SSL_RECEIVED_SHUTDOWN: c_int = 2;
//
// extern {
//     pub fn ERR_clear_error(_: c_void) -> c_void;
//
//     pub fn SSL_CTX_get_default_passwd_cb_userdata(ctx: *mut SSL_CTX) -> *mut c_void;
//     pub fn SSL_CTX_set_default_passwd_cb_userdata(ctx: *mut SSL_CTX, user_data: *mut c_void);
//     pub fn SSL_CTX_set_default_passwd_cb(ctx: *mut SSL_CTX, callback: PasswordCallback);
//     pub fn SSL_CTX_get_verify_mode(ctx: *mut SSL_CTX) -> c_int;
//     pub fn SSL_CTX_get_verify_callback(ctx: *mut SSL_CTX) -> Option<VerifyCallback>;
//     pub fn SSL_CTX_get_app_data(ctx: *mut SSL_CTX) -> *mut c_void;
//     pub fn SSL_CTX_set_app_data(ctx: *mut SSL_CTX, user_data: *mut c_void);
//     pub fn SSL_CTX_set_options(ctx: *mut SSL_CTX, options: c_long) -> c_long;
//     pub fn SSL_CTX_clear_options(ctx: *mut SSL_CTX, options: c_long) -> c_long;
//
//     pub fn SSL_get_verify_mode(ssl: *mut SSL) -> c_int;
//     pub fn SSL_set_verify_depth(ssl: *mut SSL, depth: c_int);
//     pub fn SSL_get_verify_callback(ssl: *mut SSL) -> Option<VerifyCallback>;
//     pub fn SSL_get_app_data(ssl: *mut SSL) -> *mut c_void;
//     pub fn SSL_set_app_data(ssl: *mut SSL, user_data: *mut c_void);
//     pub fn SSL_get_shutdown(ssl: *mut SSL) -> c_int;
//
//     pub fn BIO_wpending(bio: *mut BIO) -> c_int;
//     pub fn BIO_new_bio_pair(bio1: *mut *mut BIO, writebuf1: size_t,
//                             bio2: *mut *mut BIO, writebuf2: size_t) -> c_int;
//     pub fn BIO_ctrl_pending(bio: *mut BIO) -> size_t;
// }
//
// pub unsafe fn SSL_set_mode(ssl: *mut SSL, mode: c_long) -> c_long {
//     use std::ptr;
//     SSL_ctrl(ssl, SSL_CTRL_MODE, mode, ptr::null_mut())
// }
//
// pub fn clear_error() {
//     use std::mem;
//     unsafe { ERR_clear_error(mem::uninitialized()) };
// }
