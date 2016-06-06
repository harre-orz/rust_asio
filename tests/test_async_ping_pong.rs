extern crate asio;
use std::io;
use asio::*;
use asio::ip::*;
use asio::option::*;

const MESSAGE: &'static str = "hello world";

struct TcpAcceptor {
    soc: TcpListener,
}

impl TcpAcceptor {
    fn start(io: &IoService) {
        let acc = Strand::new(io, TcpAcceptor {
            soc: TcpListener::new(io, Tcp::v4()).unwrap(),
        });
        let _ = acc.soc.set_socket(&ReuseAddr(1));
        acc.soc.bind(&TcpEndpoint::new((IpAddrV4::new(127,0,0,1), 12345))).unwrap();
        acc.soc.listen().unwrap();
        TcpListener::async_accept(|acc| &acc.soc, Self::on_accept, &acc);
    }

    fn on_accept(_: Strand<TcpAcceptor>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
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
        let io = soc.io_service();
        let sv = Strand::new(&io, TcpServer {
            soc: soc,
            buf: [0; 256],
        });
        TcpSocket::async_recv(|sv| (&sv.soc, &mut sv.buf), 0, Self::on_recv, &sv);
    }

    fn on_recv(sv: Strand<TcpServer>, res: io::Result<usize>) {
        if let Ok(len) = res {
            assert!(len == MESSAGE.len());
            TcpSocket::async_send(|sv| (&sv.soc, &sv.buf[..MESSAGE.len()]), 0, Self::on_send, &sv);
        } else {
            panic!();
        }
    }

    fn on_send(_: Strand<TcpServer>, res: io::Result<usize>) {
        if let Ok(len) = res {
            assert!(len == MESSAGE.len())
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
        let _ = cl.soc.set_socket(&ReuseAddr(1));
        let ep = TcpEndpoint::new((IpAddrV4::new(127,0,0,1), 12345));
        TcpSocket::async_connect(|cl| &cl.soc, &ep, Self::on_connect, &cl);
    }

    fn on_connect(cl: Strand<TcpClient>, res: io::Result<()>) {
        if let Ok(_) = res {
            TcpSocket::async_send(|cl| (&cl.soc, MESSAGE.as_bytes()), 0, Self::on_send, &cl);
        } else {
            panic!();
        }
    }

    fn on_send(cl: Strand<TcpClient>, res: io::Result<usize>) {
        if let Ok(len) = res {
            assert!(len == MESSAGE.len());
            TcpSocket::async_recv(|cl| (&cl.soc, &mut cl.buf), 0, Self::on_recv, &cl);
        } else {
            panic!();
        }
    }

    fn on_recv(_: Strand<TcpClient>, res: io::Result<usize>) {
        if let Ok(len) = res {
            assert!(len == MESSAGE.len());
        } else {
            panic!();
        }
    }
}

#[test]
fn tests_async_ping_pong() {
    let io = IoService::new();
    TcpAcceptor::start(&io);
    TcpClient::start(&io);
    io.run();
}
