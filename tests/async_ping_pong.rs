extern crate asio;
use std::io;
use std::sync::Arc;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;

const MESSAGE: &'static str = "hello world";

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
            TcpServer::start(soc);
        } else {
            panic!();
        }
    }
}

struct TcpServer {
    soc: TcpSocket,
    buf: [u8; 256],
}

impl TcpServer {
    fn start(soc: TcpSocket) {
        let io = soc.io_service().clone();
        let sv = Strand::new(&io, TcpServer {
            soc: soc,
            buf: [0; 256],
        });
        sv.soc.async_read_some(unsafe { &mut sv.get().buf }, sv.wrap(Self::on_recv));
    }

    fn on_recv(sv: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            assert_eq!(len, MESSAGE.len());
            sv.soc.async_write_some(&sv.buf[..MESSAGE.len()], sv.wrap(Self::on_send));
        } else {
            panic!();
        }
    }

    fn on_send(_: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            assert_eq!(len, MESSAGE.len())
        } else {
            panic!();
        }
    }
}

struct TcpClient {
    soc: TcpSocket,
    buf: [u8; 256],
}

impl TcpClient {
    fn start(io: &IoService) {
        let cl = Strand::new(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
            buf: [0; 256],
        });
        let ep = TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345);
        cl.soc.async_connect(&ep, cl.wrap(Self::on_connect));
    }

    fn on_connect(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            cl.soc.async_write_some(MESSAGE.as_bytes(), cl.wrap(Self::on_send));
        } else {
            panic!();
        }
    }

    fn on_send(cl: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            assert_eq!(len, MESSAGE.len());
            cl.soc.async_read_some(unsafe { &mut cl.get().buf }, cl.wrap(Self::on_recv));
        } else {
            panic!();
        }
    }

    fn on_recv(_: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            assert_eq!(len, MESSAGE.len());
            unsafe { goal_flag = true; }
        } else {
            panic!();
        }
    }
}

#[test]
fn main() {
    let io = &IoService::new();
    TcpAcceptor::start(io);
    TcpClient::start(io);
    io.run();
    assert!(unsafe { goal_flag });
}
