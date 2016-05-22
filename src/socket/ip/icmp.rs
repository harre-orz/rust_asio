use std::io;
use std::mem;
use {IoObject, IoService, Strand};
use backbone::EpollIoActor;
use socket::{Protocol, Endpoint, ReadWrite, ResolveQuery, Resolver, SocketBase, RawSocket};
use ops::*;
use ops::async::*;
use super::IpEndpoint;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Icmp {
    family: i32,
    protocol: i32,
}

impl Icmp {
    pub fn v4() -> Icmp {
        Icmp { family: AF_INET as i32, protocol: IPPROTO_ICMP }
    }

    pub fn v6() -> Icmp {
        Icmp { family: AF_INET6 as i32, protocol: IPPROTO_ICMPV6 }
    }
}

impl Protocol for Icmp {
    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_RAW as i32
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }
}

impl Endpoint<Icmp> for IpEndpoint<Icmp> {
    fn protocol(&self) -> Icmp {
        if self.is_v4() {
            Icmp::v4()
        } else if self.is_v6() {
            Icmp::v6()
        } else {
            unreachable!("Invalid family code ({}).", self.ss.ss_family);
        }
    }
}

pub type IcmpEndpoint = IpEndpoint<Icmp>;

pub struct IcmpSocket {
    io: IoService,
    actor: EpollIoActor,
}

impl Drop for IcmpSocket {
    fn drop(&mut self) {
        let _ = self.actor.unset_in(&self.io);
        let _ = self.actor.unset_out(&self.io);
        let _ = close(self);
    }
}

impl IoObject for IcmpSocket {
    fn io_service(&self) -> IoService {
        self.io.clone()
    }
}

impl AsRawFd for IcmpSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.actor.as_raw_fd()
    }
}

impl AsIoActor for IcmpSocket {
    fn as_io_actor(&self) -> &EpollIoActor {
        &self.actor
    }
}

impl SocketBase<Icmp> for IcmpSocket {
    type Endpoint = IcmpEndpoint;

    fn new(io: &IoService, pro: Icmp) -> io::Result<Self> {
        let fd = try!(socket(pro));
        Ok(IcmpSocket {
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

impl RawSocket<Icmp> for IcmpSocket {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()> {
        connect(self, ep)
    }

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send + 'static,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send + 'static,
              T: 'static {
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

pub struct IcmpResolver {
    io: IoService,
}

impl IoObject for IcmpResolver {
    fn io_service(&self) -> IoService {
        self.io.clone()
    }
}

impl Resolver<Icmp> for IcmpResolver {
    fn new(io: &IoService) -> Self {
        IcmpResolver {
            io: io.clone(),
        }
    }

    fn resolve<'a, Q: ResolveQuery<'a, Icmp>>(&self, query: Q) -> io::Result<Q::Iter> {
        query.query(Icmp { family: AF_UNSPEC, protocol: 0 })
    }

    fn async_resolve<'a, Q, A, F, T>(a: A, query: Q, callback: F, obj: &Strand<T>)
        where Q: ResolveQuery<'a, Icmp> + 'static,
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
fn test_icmp() {
    assert!(Icmp::v4() == Icmp::v4());
    assert!(Icmp::v6() == Icmp::v6());
    assert!(Icmp::v4() != Icmp::v6());
}

#[test]
fn test_icmp_resolve() {
    use super::IpAddrV4;
    let io = IoService::new();
    let re = IcmpResolver::new(&io);
    for e in re.resolve(("127.0.0.1", "")).unwrap() {
        assert!(e.endpoint() == IcmpEndpoint::new((IpAddrV4::new(127,0,0,1), 0)));
    }
}
