use unsafe_cell::UnsafeRefCell;
use error::{ErrCode, eof};
use core::{IoContext, AsIoContext};
use async::Handler;
use streams::Stream;
use buffers::StreamBuf;
use ssl::*;
use ssl::ffi::*;

use std::io;
use std::ptr;
use std::sync::Mutex;
use libc::{c_void, c_int, size_t};
use openssl_sys::*;
use openssl::types::OpenSslTypeRef;

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

enum Want {
    InputAndRetry = -2,

    OutputAndRetry = -1,

    Nothing = 0,

    Output = 1,
}

struct Engine {
    ssl: *mut SSL,
    ext_bio: *mut BIO,
    verify_callback: Box<Fn(bool, &SslVerifyContext) -> bool>,
}

impl Engine {
    pub fn new(ctx: &SslContext) -> Result<Engine> {
        let ssl = unsafe { SSL_new(ctx.as_ptr()) };
        if ssl.is_null() {
            return Err(Error::last_ssl_error());
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

    pub fn set_verify_mode(&self, mode: SslVerifyMode) -> Result<()> {
        unsafe { SSL_set_verify(self.ssl, mode.bits(), SSL_get_verify_callback(self.ssl)) };
        Ok(())
    }

    pub fn set_verify_depth(&self, depth: i32) -> Result<()> {
        unsafe { SSL_set_verify_depth(self.ssl, depth as c_int) };
        Ok(())
    }

    extern "C" fn verify_callback(preverified: c_int, ctx: *mut X509_STORE_CTX) -> c_int {
        if !ctx.is_null() {
            unsafe {
                let ssl = X509_STORE_CTX_get_ex_data(ctx, SSL_get_ex_data_X509_STORE_CTX_idx()) as *mut SSL;
                if !ssl.is_null() {
                    let this = &*(SSL_get_app_data(ssl) as *const Self);
                    return (*this.verify_callback)(preverified != 0, SslVerifyContext::from_ptr(ctx)) as c_int;
                }
            }
        }
        0
    }

    pub fn set_verify_callback<F>(&mut self, callback: F) -> Result<()>
        where F: Fn(bool, &SslVerifyContext) -> bool + 'static
    {
        let user_data = self as *mut Self;
        self.verify_callback = Box::new(callback);
        unsafe {
            SSL_set_app_data(self.ssl, user_data as *mut c_void);
            SSL_set_verify(self.ssl, SSL_get_verify_mode(self.ssl), Some(Self::verify_callback));
        }
        Ok(())
    }

    fn perform(&self, op: fn(*mut SSL, *mut c_void, size_t) -> c_int, data: *mut c_void, len: size_t) -> (Want, Result<usize>) {
        let pending_output_before = unsafe { BIO_ctrl_pending(self.ext_bio) };
        clear_error();
        let res = op(self.ssl, data, len);
        let err = unsafe { SSL_get_error(self.ssl, res) };
        let pending_output_after = unsafe { BIO_ctrl_pending(self.ext_bio) };

        if err == SSL_ERROR_SSL {
            (Want::Nothing, Err(Error::last_ssl_error()))
        }
        else if err == SSL_ERROR_SYSCALL {
            (Want::Nothing, Err(Error::last_sys_error()))
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

    pub fn handshake(&self, mode: Handshake) -> (Want, Result<usize>) {
        self.perform(match mode {
            Handshake::Client => do_connect,
            Handshake::Server => do_accept,
        }, ptr::null_mut(), 0)
    }

    pub fn shutdown(&self) -> (Want, Result<usize>) {
        self.perform(do_shutdown, ptr::null_mut(), 0)
    }

    pub fn read(&self, buf: &mut [u8]) -> (Want, Result<usize>) {
        if buf.len() == 0 {
            (Want::Nothing, Ok(0))
        } else {
            self.perform(do_read, buf.as_ptr() as *mut c_void, buf.len())
        }
    }

    pub fn write(&self, buf: &[u8]) -> (Want, Result<usize>) {
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

struct SslHandler<S, O, F> {
    imp: UnsafeRefCell<SslStreamImpl>,
    next_layer: UnsafeRefCell<S>,
    op: O,
    handler: F,
}

// impl<S, O, F> Handler<usize, io::Error> for SslHandler<S, O, F>
//     where S: Stream<io::Error>,
//           O: FnMut(&Engine) -> (Want, Result<usize>) + Send + 'static,
//           F: Handler<usize, io::Error>,
// {
//     type Output = F::Output;

//     fn callback(self, io: &IoService, res: ::std::result::Result<usize, io::Error>) {
//     //     let SslHandler { mut imp, next_layer, mut op, handler } = self;
//     //     let mut imp = unsafe { imp.as_mut() };
//     //     let imp_clone = UnsafeRefCell::new(imp);
//     //     let next_layer = unsafe { next_layer.as_ref() };

//     //     match res {
//     //         Ok(start) if start == 1 => {
//     //             loop {
//     //                 match op(&imp.engine) {
//     //                     (Want::InputAndRetry, _) => {
//     //                         if imp.input_buf.len() != 0 {
//     //                             imp.engine.put_input(&mut imp.input_buf);
//     //                         } else {
//     //                             match imp.input_buf.prepare(4096) {
//     //                                 Ok(buf) => {
//     //                                     let handler = SslHandler {
//     //                                         imp: imp_clone,
//     //                                         next_layer: UnsafeRefCell::new(next_layer),
//     //                                         op: op,
//     //                                         handler: handler,
//     //                                     };
//     //                                     next_layer.async_read_some(buf, handler);
//     //                                 }
//     //                                 Err(err) => handler.callback(io, Err(err.into())),
//     //                             }
//     //                             return;
//     //                         }
//     //                     },
//     //                     (Want::OutputAndRetry, _) | (Want::Output, _) => {
//     //                         let handler = SslHandler {
//     //                             imp: imp_clone,
//     //                             next_layer: UnsafeRefCell::new(next_layer),
//     //                             op: op,
//     //                             handler: handler,
//     //                         };
//     //                         let len = imp.output_buf.len();
//     //                         async_write_until(next_layer, &mut imp.output_buf, len, handler);
//     //                         return;
//     //                     },
//     //                     _ => {
//     //                         if start > 0 {

//     //                         }
//     //                     },
//     //                 }
//     //             }
//     //         },
//     //         Ok(_) => {
//     //         },
//     //         Err(err) => return handler.callback(io, Err(err.into())),
//     //     }
//     }

//     fn wrap<G>(self, callback: G) -> Callback
//         where G: FnOnce(&IoService, ErrCode, Self) + Send + 'static,
//     {
//         let SslHandler { imp, next_layer, op, handler } = self;
//         handler.wrap(move |io, ec, handler| {
//             callback(io, ec, SslHandler {
//                 imp: imp,
//                 next_layer: next_layer,
//                 op: op,
//                 handler: handler,
//             })
//         })
//     }

//     type AsyncResult = F::AsyncResult;

//     fn async_result(&self) -> Self::AsyncResult {
//         self.handler.async_result()
//     }
// }

struct SslStreamImpl {
    engine: Engine,
    input_buf: StreamBuf,
    output_buf: StreamBuf,
}

impl SslStreamImpl {
    fn new(ctx: &SslContext) -> Result<SslStreamImpl> {
        Ok(SslStreamImpl {
            engine: try!(Engine::new(ctx)),
            input_buf: StreamBuf::new(),
            output_buf: StreamBuf::new(),
        })
    }

    fn io_mut<S, F>(&mut self, next_layer: &S, mut op: F) -> Result<usize>
        where S: Stream<io::Error>,
              F: FnMut(&Engine) -> (Want, Result<usize>),
    {
        // loop {
        //     match op(&self.engine) {
        //         (Want::InputAndRetry, _) => {
        //             if self.input_buf.len() == 0 {
        //                 let len = try!(next_layer.read_some(try!(self.input_buf.prepare(4096))));
        //                 self.input_buf.commit(len);
        //             }
        //             self.engine.put_input(&mut self.input_buf);
        //         },
        //         (Want::OutputAndRetry, _) => {
        //             self.engine.get_output(&mut self.output_buf);
        //             let len = self.output_buf.len();
        //             if len > 0 {
        //                 try!(write_until(next_layer, &mut self.output_buf, len));
        //             }
        //         },
        //         (Want::Output, res) => {
        //             self.engine.get_output(&mut self.output_buf);
        //             let len = self.output_buf.len();
        //             try!(write_until(next_layer, &mut self.output_buf, len));
        //             return res;
        //         },
        //         (_, res) => if let Ok(len) = res {
        //             return Ok(len);
        //         },
        //     }
        // }
        return Ok(0)
    }

    fn io<S, O>(&self, next_layer: &S, op: O) -> Result<usize>
        where S: Stream<io::Error>,
              O: FnMut(&Engine) -> (Want, Result<usize>),
    {
        unsafe { &mut *(self as *const _ as *mut Self) }.io_mut(next_layer, op)
    }
}

unsafe impl Send for SslStreamImpl {
}

unsafe impl Sync for SslStreamImpl {
}

pub struct SslStream<S> {
    soc: S,
    core: SslStreamImpl,
    _ctx: SslContext,
}

impl<S: Stream<io::Error>> SslStream<S> {
    pub fn new(soc: S, ctx: &SslContext) -> Result<SslStream<S>> {
        let core = try!(SslStreamImpl::new(ctx));
        Ok(SslStream {
            soc: soc,
            core: core,
            _ctx: ctx.clone(),
        })
    }

    pub fn async_handshake(&self) {
    }

    pub fn async_shutdown(&self) {
    }

    pub fn handshake(&self, mode: Handshake) -> Result<()> {
        match self.core.io(&self.soc, move |eng| eng.handshake(mode)) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn next_layer(&self) -> &S {
        &self.soc
    }

    pub fn set_verify_callback<F>(&mut self, callback: F) -> Result<()>
        where F: Fn(bool, &SslVerifyContext) -> bool + 'static
    {
        self.core.engine.set_verify_callback(callback)
    }

    pub fn set_verify_depth(&self, depth: i32) -> Result<()> {
        self.core.engine.set_verify_depth(depth)
    }

    pub fn set_verify_mode(&self, mode: SslVerifyMode) -> Result<()> {
        self.core.engine.set_verify_mode(mode)
    }

    pub fn shutdown(&mut self) -> Result<()> {
        match self.core.io(&self.soc, |eng| eng.shutdown()) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

unsafe impl<S: Stream<io::Error>> AsIoContext for SslStream<S> {
    fn as_ctx(&self) -> &IoContext {
        self.soc.as_ctx()
    }
}

// impl<S: Stream<io::Error>> Stream<Error> for SslStream<S> {
//     fn async_read_some<F: Handler<usize, Error>>(&self, buf: &mut [u8], handler: F) -> F::Output {
//         handler.async_result().get(self.io_service())
//     }

//     fn async_write_some<F: Handler<usize, Error>>(&self, buf: &[u8], handler: F) -> F::Output {
//         handler.async_result().get(self.io_service())
//     }

//     fn read_some(&self, buf: &mut [u8]) -> Result<usize> {
//         self.core.io(&self.soc, |eng| eng.read(buf))
//     }

//     fn write_some(&self, buf: &[u8]) -> Result<usize> {
//         self.core.io(&self.soc, |eng| eng.write(buf))
//     }
// }
