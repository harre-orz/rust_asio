use std::io;
use std::mem;
use {IoObject, IoService, Strand};
use backbone::EpollIoActor;
use socket::{Protocol, Endpoint, ReadWrite, ResolveQuery, Resolver, SocketBase, DgramSocket};
use ops::*;
use ops::async::*;
use super::IpEndpoint;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Udp {
    family: i32,
}

impl Udp {
    pub fn v4() -> Udp {
        Udp { family: AF_INET as i32 }
    }

    pub fn v6() -> Udp {
        Udp { family: AF_INET6 as i32 }
    }
}

impl Protocol for Udp {
    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_DGRAM as i32
    }

    fn protocol_type(&self) -> i32 {
        0
    }
}

impl Endpoint<Udp> for IpEndpoint<Udp> {
    fn protocol(&self) -> Udp {
        if self.is_v4() {
            Udp::v4()
        } else if self.is_v6() {
            Udp::v6()
        } else {
            unreachable!("Invalid family code ({}).", self.ss.ss_family);
        }
    }
}

pub type UdpEndpoint = IpEndpoint<Udp>;

pub struct UdpSocket {
    io: IoService,
    actor: EpollIoActor,
}

impl Drop for UdpSocket {
    fn drop(&mut self) {
        let _ = self.actor.unset_in(&self.io);
        let _ = self.actor.unset_out(&self.io);
        let _ = close(self);
    }
}

impl IoObject for UdpSocket {
    fn io_service(&self) -> IoService {
        self.io.clone()
    }
}

impl AsRawFd for UdpSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for UdpSocket {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl SocketBase<Udp> for UdpSocket {
    type Endpoint = UdpEndpoint;

    fn new(io: &IoService, pro: Udp) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(UdpSocket {
            io: io.clone(),
            actor: EpollIoActor::new(fd),
        })
    }

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()> {
        bind(self, ep)
    }

    fn local_endpoint(&self) -> io::Result<Self::Endpoint> {
        getsockname(self, unsafe { mem::uninitialized() })
    }
}

impl DgramSocket<Udp> for UdpSocket {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()> {
        connect(self, ep)
    }

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static
    {
        async_connect(a, ep, callback, obj)
    }

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint> {
        getpeername(self, unsafe { mem::uninitialized() })
    }

    fn recv(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        recv(self, buf, flags)
    }

    fn async_recv<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        async_recv(a, flags, callback, obj)
    }

    fn recv_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, Self::Endpoint)> {
        recvfrom(self, buf, flags, unsafe { mem::uninitialized() })
    }

    fn async_recv_from<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<(usize, Self::Endpoint)>) + Send + 'static,
              T: 'static {
        async_recvfrom(a, flags, unsafe { mem::uninitialized() }, callback, obj)
    }

    fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize> {
        send(self, buf, flags)
    }

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        async_send(a, flags, callback, obj)
    }

    fn send_to(&self, buf: &[u8], flags: i32, ep: &Self::Endpoint) -> io::Result<usize> {
        sendto(self, buf, flags, ep)
    }

    fn async_send_to<A, F, T>(a: A, flags: i32, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
              T: 'static {
        async_sendto(a, flags, ep, callback, obj)
    }

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self {
        cancel_io(a, obj)
    }
}

pub struct UdpResolver {
    io: IoService,
}

impl IoObject for UdpResolver {
    fn io_service(&self) -> IoService {
        self.io.clone()
    }
}

impl Resolver<Udp> for UdpResolver {
    fn new(io: &IoService) -> Self {
        UdpResolver {
            io: io.clone(),
        }
    }

    fn resolve<'a, Q: ResolveQuery<'a, Udp>>(&self, query: Q) -> io::Result<Q::Iter> {
        query.query(Udp { family: AF_UNSPEC })
    }

    fn async_resolve<'a, Q, A, F, T>(a: A, query: Q, callback: F, obj: &Strand<T>)
        where Q: ResolveQuery<'a, Udp> + 'static,
              A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<Q::Iter>) + Send + 'static,
              T: 'static {
        let io = a(&*obj).io_service();
        let _obj = obj.clone();
        io.post_strand(move || {
            let res = a(&*_obj);
            callback(&_obj, res.resolve(query));
        }, obj)
    }
}

#[test]
fn test_udp() {
    assert!(Udp::v4() == Udp::v4());
    assert!(Udp::v6() == Udp::v6());
    assert!(Udp::v4() != Udp::v6());
}

#[test]
fn test_udp_resolve() {
    use super::IpAddrV4;
    let io = IoService::new();
    let re = UdpResolver::new(&io);
    for e in re.resolve(("127.0.0.1", "80")).unwrap() {
        assert!(e.endpoint() == UdpEndpoint::new((IpAddrV4::new(127,0,0,1), 80)));
    }
}
