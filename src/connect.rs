use std::io;
use std::iter::Iterator;
use {IoObject, IoService, Protocol, Endpoint, FromRawFd, Handler};
use backbone::{socket};

fn host_not_found() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Host not found")
}

pub trait Connect<P: Protocol> : IoObject + FromRawFd<P> + Send + 'static {
    fn async_connect<F: Handler<()>>(&self, ep: &P::Endpoint, handler: F);

    fn connect(&self, ep: &P:: Endpoint) -> io::Result<()>;
}

pub fn connect<T, P, S, I>(io: &T, it: I) -> io::Result<(S, P::Endpoint)>
    where T: IoObject,
          P: Protocol,
          S: Connect<P>,
          I: Iterator<Item=P::Endpoint>,
{
    for ep in it {
        let pro = ep.protocol();
        let fd = try!(socket(&pro));
        let soc = unsafe { S::from_raw_fd(io.io_service(), pro, fd) };
        if let Ok(_) = soc.connect(&ep) {
            return Ok((soc, ep));
        }
    }
    Err(host_not_found())
}

struct ConnectHandler<P: Protocol, S, I, F> {
    ptr: Box<(S, P::Endpoint)>,
    it: I,
    handler: F,
}

impl<P, S, I, F> Handler<()> for ConnectHandler<P, S, I, F>
    where P: Protocol,
          S: Connect<P>,
          I: Iterator<Item=P::Endpoint> + Send + 'static,
          F: FnOnce(io::Result<(S, P::Endpoint)>) + Send + 'static,
{
    fn callback(self, io: &IoService, res: io::Result<()>) {
        let ConnectHandler { ptr, it, handler } = self;
        match res {
            Ok(_) => handler(Ok(*ptr)),
            _ => async_connect(io, it, handler),
        }
    }
}

pub fn async_connect<T, P, S, I, F>(io: &T, mut it: I, handler: F)
    where T: IoObject,
          P: Protocol,
          S: Connect<P>,
          I: Iterator<Item=P::Endpoint> + Send + 'static,
          F: FnOnce(io::Result<(S, P::Endpoint)>) + Send + 'static,
{
    match it.next() {
        Some(ep) => {
            let pro = ep.protocol();
            match socket(&pro) {
                Ok(fd) => {
                    let handler = ConnectHandler {
                        ptr: Box::new((unsafe { S::from_raw_fd(io, pro, fd) }, ep)),
                        it: it,
                        handler: handler,
                    };
                    let soc = unsafe { &*(&handler.ptr.0 as *const S) };
                    let ep = unsafe { &*(&handler.ptr.1 as *const P::Endpoint) };
                    soc.async_connect(ep, handler);
                },
                Err(err) => handler(Err(err)),
            }
        },
        _ => handler(Err(host_not_found())),
    }
}
