use prelude::{SockAddr};
use ffi::{RawFd, AsRawFd, close, socket, bind, listen, accept, connect, write,
          ioctl, setsockopt, getsockname, INVALID_SOCKET};
use core::{Reactor, IntrFd};
use socket_base::{MAX_CONNECTIONS, NonBlockingIo, ReuseAddr};
use ip::{IpProtocol, IpAddrV4, Tcp, TcpEndpoint, NoDelay};

use std::io;

struct SocketHolder(RawFd);

impl Drop for SocketHolder {
    fn drop(&mut self) {
        close(self.as_raw_fd())
    }
}

impl AsRawFd for SocketHolder {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

pub struct SocketInterrupter {
    tcp_wfd: IntrFd,
    tcp_rfd: IntrFd,
}

impl SocketInterrupter {
    pub fn new() -> io::Result<Self> {
        let pro = Tcp::v4();
        let ep = TcpEndpoint::new(IpAddrV4::loopback(), 0);
        let acc = SocketHolder(try!(socket(&pro)));
        try!(setsockopt(&acc, &pro, ReuseAddr::new(true)));
        try!(bind(&acc, &ep));
        let mut ep = try!(getsockname(&acc, &pro));

        // Refer: socket_select_interrupter.ipp of boost::asio
        // 0.0.0.0 で取得したときファイヤーウォールの警告になるのを回避するために
        // 127.0.0.1 を代入しているようだ
        if ep.addr().is_unspecified() {
            ep = TcpEndpoint::new(IpAddrV4::loopback(), ep.port());
        }
        try!(listen(&acc, MAX_CONNECTIONS));

        let cl = IntrFd::new::<Self>(try!(socket(&pro)));
        libc_try!(connect(&cl, &ep));
        try!(setsockopt(&cl, &pro, NoDelay::new(true)));
        try!(ioctl(&cl, &mut NonBlockingIo::new(true)));

        let mut sa_len = ep.capacity() as _;
        let sv = unsafe { accept(&acc, &mut ep, &mut sa_len) };
        if sv == INVALID_SOCKET {
            return Err(io::Error::last_os_error());
        }
        let sv = IntrFd::new::<Self>(sv);
        try!(setsockopt(&sv, &pro, NoDelay::new(true)));
        try!(ioctl(&sv, &mut NonBlockingIo::new(true)));

        Ok(SocketInterrupter {
            tcp_rfd: sv,
            tcp_wfd: cl,
        })
    }

    pub fn startup(&self, ctx: &Reactor) {
        ctx.register_intr_fd(&self.tcp_rfd)
    }

    pub fn cleanup(&self, ctx: &Reactor) {
        ctx.deregister_intr_fd(&self.tcp_rfd)
    }
    
    pub fn interrupt(&self) {
        let buf = [1];
        libc_ign!(write(&self.tcp_wfd, &buf));
    }
}

