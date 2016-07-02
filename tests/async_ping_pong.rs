extern crate asio;
use std::io;
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
        let acc = Strand::new(io, TcpAcceptor {
            soc: TcpListener::new(io, Tcp::v4()).unwrap(),
        });
        acc.soc.set_option(ReuseAddr::new(true)).unwrap();
        acc.soc.bind(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
        acc.soc.listen().unwrap();
        acc.soc.async_accept(Self::on_accept, &acc);
    }

    fn on_accept(acc: Strand<Self>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
        if let Ok((soc, _)) = res {
            TcpServer::start(acc.io_service(), soc);
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
    fn start(io: &IoService, soc: TcpSocket) {
        let sv = Strand::new(io, TcpServer {
            soc: soc,
            buf: [0; 256],
        });
        sv.soc.async_receive(|sv| &mut sv.buf, 0, Self::on_recv, &sv);
    }

    fn on_recv(sv: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            assert_eq!(len, MESSAGE.len());
            sv.soc.async_send(|sv| &sv.buf[..MESSAGE.len()], 0, Self::on_send, &sv);
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
        cl.soc.async_connect(&ep, Self::on_connect, &cl);
    }

    fn on_connect(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            cl.soc.async_send(|_| MESSAGE.as_bytes(), 0, Self::on_send, &cl);
        } else {
            panic!();
        }
    }

    fn on_send(cl: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            assert_eq!(len, MESSAGE.len());
            cl.soc.async_receive(|cl| &mut cl.buf, 0, Self::on_recv, &cl);
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
    let io = IoService::new();
    TcpAcceptor::start(&io);
    TcpClient::start(&io);
    io.run();

    assert!(unsafe { goal_flag });
}
