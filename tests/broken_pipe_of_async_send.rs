extern crate asio;
extern crate time;
use std::io;
use std::sync::Arc;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;

static mut goal_flag: bool = false;

struct TcpAcceptor {
    soc: TcpListener,
}

impl TcpAcceptor {
    fn start(io: &IoService) {
        let acc = Arc::new(TcpAcceptor {
            soc: TcpListener::new(io, Tcp::v4()).unwrap(),
        });
        acc.soc.set_option(ReuseAddr::new(true)).unwrap();
        acc.soc.bind(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
        acc.soc.listen().unwrap();
        acc.soc.async_accept(bind(Self::on_accept, &acc));
    }

    fn on_accept(_: Arc<Self>, res: io::Result<(TcpSocket, TcpEndpoint)>, _: &IoService) {
        if let Ok((soc, _)) = res {
        } else {
            panic!();
        }
    }
}

struct TcpClient {
    soc: TcpSocket,
    buf: Vec<u8>,
}

impl TcpClient {
    fn start(io: &IoService) {
        let mut cl = Strand::new(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
            buf: Vec::with_capacity(1024*1024),
        });
        unsafe {
            let len = cl.buf.capacity();
            cl.buf.set_len(len);
        }
        let ep = TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345);
        cl.soc.async_connect(&ep, cl.wrap(Self::on_connect));
    }

    fn on_connect(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            cl.soc.async_send(cl.buf.as_slice(), 0, cl.wrap(Self::on_send));
        } else {
            panic!();
        }
    }

    fn on_send(cl: Strand<Self>, res: io::Result<usize>) {
        match res {
            Ok(_) =>
                cl.soc.async_send(cl.buf.as_slice(), 0, cl.wrap(Self::on_send)),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
                unsafe { goal_flag = true; }
            }
        }
    }
}

#[test]
fn main() {
    let io = IoService::new();
    TcpAcceptor::start(&io);
    TcpClient::start(&io);
    io.run();
    assert!(unsafe { goal_flag })
}
