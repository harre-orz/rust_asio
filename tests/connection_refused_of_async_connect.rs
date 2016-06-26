extern crate asio;
use std::io;
use asio::*;
use asio::ip::*;

struct TcpClient {
    soc: TcpSocket,
}

impl TcpClient {
    fn start(io: &IoService) {
        let cl = Strand::new(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
        });
        TcpSocket::async_connect(|cl| &cl.soc, &TcpEndpoint::new((IpAddrV4::loopback(), 12345)), Self::on_connect, &cl);
    }

    fn on_connect(_: Strand<Self>, res: io::Result<()>) {
        if let Err(err) = res {
            assert_eq!(err.kind(), io::ErrorKind::ConnectionRefused);
        } else {
            panic!();
        }
    }
}

#[test]
fn main() {
    let io = IoService::new();
    TcpClient::start(&io);
    io.run();
}
