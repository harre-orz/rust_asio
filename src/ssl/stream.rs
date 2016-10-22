use std::io;
use std::ptr;
use std::cell::UnsafeCell;
use io_service::{IoObject, IoService, Handler, AsyncResult};
use error::eof;
use stream::*;
use streambuf::StreamBuf;
use libc::{c_void, c_int, c_long, size_t};
use openssl_sys::*;
use super::*;
use super::ffi::*;

struct SslStreamImpl {
    engine: Engine,
    input_buf: StreamBuf,
    output_buf: StreamBuf,
}

impl SslStreamImpl {
    fn new(ctx: &SslContext) -> SslResult<SslStreamImpl> {
        Ok(SslStreamImpl {
            engine: try!(Engine::new(ctx)),
            input_buf: StreamBuf::new(),
            output_buf: StreamBuf::new(),
        })
    }

    fn io_mut<S, F>(&mut self, next_layer: &S, mut op: F) -> SslResult<usize>
        where S: Stream<io::Error>,
              F: FnMut(&Engine) -> (Want, SslResult<usize>),
    {
        loop {
            match op(&self.engine) {
                (Want::InputAndRetry, _) => {
                    if self.input_buf.len() == 0 {
                        let len = try!(next_layer.read_some(try!(self.input_buf.prepare(4096))));
                        self.input_buf.commit(len);
                    }
                    self.engine.put_input(&mut self.input_buf);
                },
                (Want::OutputAndRetry, _) => {
                    self.engine.get_output(&mut self.output_buf);
                    let len = self.output_buf.len();
                    if len > 0 {
                        try!(write_until(next_layer, &mut self.output_buf, len));
                    }
                },
                (Want::Output, res) => {
                    self.engine.get_output(&mut self.output_buf);
                    let len = self.output_buf.len();
                    try!(write_until(next_layer, &mut self.output_buf, len));
                    return res;
                },
                (_, res) => if let Ok(len) = res { return res; },
            }
        }
    }

    fn io<S, F>(&self, next_layer: &S, mut op: F) -> SslResult<usize>
        where S: Stream<io::Error>,
              F: FnMut(&Engine) -> (Want, SslResult<usize>),
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
    pub fn new(soc: S, ctx: &SslContext) -> SslResult<SslStream<S>> {
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

    pub fn handshake(&self, mode: Handshake) -> SslResult<()> {
        match self.core.io(&self.soc, move |eng| eng.handshake(mode)) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn next_layer(&self) -> &S {
        &self.soc
    }

    pub fn set_verify_callback<F>(&mut self, callback: F) -> SslResult<()>
        where F: Fn(bool, SslVerifyContext) -> bool + 'static
    {
        self.core.engine.set_verify_callback(callback)
    }

    pub fn set_verify_depth(&self, depth: i32) -> SslResult<()> {
        self.core.engine.set_verify_depth(depth)
    }

    pub fn set_verify_mode(&self, mode: VerifyMode) -> SslResult<()> {
        self.core.engine.set_verify_mode(mode)
    }

    pub fn shutdown(&mut self) -> SslResult<()> {
        match self.core.io(&self.soc, |eng| eng.shutdown()) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

unsafe impl<S: Stream<io::Error>> IoObject for SslStream<S> {
    fn io_service(&self) -> &IoService {
        self.soc.io_service()
    }
}

impl<S: Stream<io::Error>> Stream<SslError> for SslStream<S> {
    fn async_read_some<F: Handler<usize, SslError>>(&self, buf: &mut [u8], handler: F) -> F::Output {
        handler.async_result().get(self.io_service())
    }

    fn async_write_some<F: Handler<usize, SslError>>(&self, buf: &[u8], handler: F) -> F::Output {
        handler.async_result().get(self.io_service())
    }

    fn read_some(&self, buf: &mut [u8]) -> SslResult<usize> {
        self.core.io(&self.soc, |eng| eng.read(buf))
    }

    fn write_some(&self, buf: &[u8]) -> SslResult<usize> {
        self.core.io(&self.soc, |eng| eng.write(buf))
    }
}
