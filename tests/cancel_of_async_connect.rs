extern crate asio;
extern crate time;
use std::io;
use time::Duration;
use asio::*;
use asio::ip::*;

struct TcpClient {
    soc: TcpSocket,
    timer: SteadyTimer,
}

impl TcpClient {
    fn start(io: &IoService) {
        let cl = Strand::new(io, TcpClient {
            soc: TcpSocket::new(Tcp::v4()).unwrap(),
            timer: SteadyTimer::new(),
        });
        SteadyTimer::async_wait_for(|cl| &cl.timer, &Duration::milliseconds(1000), Self::on_wait, &cl);
        TcpSocket::async_connect(|cl| &cl.soc, &TcpEndpoint::new((IpAddrV4::new(192,0,2,1), 12345)), Self::on_connect, &cl);
    }

    fn on_wait(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            TcpSocket::cancel(|cl| &cl.soc, &cl);
        } else {
            panic!()
        }
    }

    fn on_connect(_: Strand<Self>, res: io::Result<()>) {
        if let Err(err) = res {
            assert_eq!(err.kind(), io::ErrorKind::Other);  // Cancel
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
