use prelude::{Protocol, Socket};
use core::AsIoContext;
use async::Handler;

use std::io;


pub trait Stream<P> : Socket<P> + AsIoContext + io::Read + io::Write
    where P: Protocol,
{
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>;

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>;

    // fn async_read_until<F>(&self, buf: &mut [u8], handler: F) -> F::Output
    //     where F: Handler<usize, io::Error>
    // {
    //     let (tx, rx) = handler.channel();
    //     self.async_read_some(buf, AsyncReadUntil::new(self, tx));
    //     rx.yield_return(self.as_ctx())
    // }
}
