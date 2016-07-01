extern crate asio;
use std::io;
use asio::*;
use asio::ip::*;

static mut goal_flag: bool = false;

struct TcpClient {
    soc: TcpSocket,
}

impl TcpClient {
    fn start(io: &IoService) {
        let cl = Strand::new(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
        });
        cl.soc.async_connect(&TcpEndpoint::new(IpAddrV4::loopback(), 12345), Self::on_connect, &cl);
    }

    fn on_connect(_: Strand<Self>, res: io::Result<()>) {
        if let Err(err) = res {
            assert_eq!(err.kind(), io::ErrorKind::ConnectionRefused);
            unsafe { goal_flag = true; }
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
    assert!(unsafe { goal_flag })
}
