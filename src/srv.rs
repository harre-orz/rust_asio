use super::*;
use super::ops;
use std::io;
use std::mem;
use std::sync::{Arc, Mutex};
use libc;

struct TakoObject {
    timeout: i32,
}

pub struct TakoIoService {
    srv: Arc<Mutex<TakoObject>>
}

impl TakoIoService {
    pub fn default() -> IoService {
        IoService { srv: Arc::new(Mutex::new(TakoObject { timeout: 10000 })) }
    }

    pub fn connect<'a, P: Protocol, S: StreamSocket<'a>, E: Endpoint<P>>(&self, mut soc: S, ep: &E) -> io::Result<S> {
        let tako = self.srv.lock().unwrap();
        soc.set_nonblocking(true);
        if unsafe { libc::connect(*soc.native_handle(), ep.as_sockaddr(), ep.socklen()) == 0 } {
            soc.set_nonblocking(false);
            return Ok(soc);
        }

        if let Some(errno) = io::Error::last_os_error().raw_os_error() {
            if errno == 115 {
                let mut fd = libc::pollfd { fd: unsafe { *soc.native_handle() }, events: libc::POLLOUT, revents: 0 };
                match libc_try!(libc::poll(mem::transmute(&mut fd), 1, tako.timeout)) {
                    0 => return Err(io::Error::new(io::ErrorKind::Other, "timed out")),
                    _ => {
                        soc.set_nonblocking(false);
                        return Ok(soc);
                    }
                }
            }
        }
        Err(io::Error::last_os_error())
    }

    pub fn receive<'a, S: Socket<'a>, B: MutableBuffer>(&self, soc: &mut S, buf: B) -> io::Result<usize> {
        let tako = self.srv.lock().unwrap();
        let mut fd = libc::pollfd { fd: unsafe { *soc.native_handle() }, events: libc::POLLIN, revents: 0 };
        match libc_try!(libc::poll(mem::transmute(&mut fd), 1, tako.timeout)) {
            0 => Err(io::Error::new(io::ErrorKind::Other, "timed out")),
            _ => ops::receive(soc, buf),
        }
    }

    pub fn receive_from<'a, P: Protocol, S: Socket<'a>, B: MutableBuffer, E: Endpoint<P>>(&self, soc: &mut S, buf: B, ep: &mut E) -> io::Result<usize> {
        let tako = self.srv.lock().unwrap();
        let mut fd = libc::pollfd { fd: unsafe { *soc.native_handle() }, events: libc::POLLIN, revents: 0 };
        match libc_try!(libc::poll(mem::transmute(&mut fd), 1, tako.timeout)) {
            0 => Err(io::Error::new(io::ErrorKind::Other, "timed out")),
            _ => ops::receive_from(soc, buf, ep),
        }
    }

    pub fn send<'a, S: Socket<'a>, B: Buffer>(&self, soc: &mut S, buf: B) -> io::Result<usize> {
        let tako = self.srv.lock().unwrap();
        let mut fd = libc::pollfd { fd: unsafe { *soc.native_handle() }, events: libc::POLLOUT, revents: 0 };
        match libc_try!(libc::poll(mem::transmute(&mut fd), 1, tako.timeout)) {
            0 => Err(io::Error::new(io::ErrorKind::Other, "timed out")),
            _ => ops::send(soc, buf),
        }
    }

    pub fn send_to<'a, P: Protocol, S: Socket<'a>, B: Buffer, E: Endpoint<P>>(&self, soc: &mut S, mut buf: B, ep: &E) -> io::Result<usize> {
        let tako = self.srv.lock().unwrap();
        let mut fd = libc::pollfd { fd: unsafe { *soc.native_handle() }, events: libc::POLLOUT, revents: 0 };
        match libc_try!(libc::poll(mem::transmute(&mut fd), 1, tako.timeout)) {
            0 => Err(io::Error::new(io::ErrorKind::Other, "timed out")),
            _ => ops::send_to(soc, buf, ep),
        }
    }
}
