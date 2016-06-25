extern crate asio;
extern crate time;
use std::io;
use time::Duration;
use asio::*;
use asio::ip::*;

struct UdpClient {
    soc: UdpSocket,
    timer: SteadyTimer,
    buf: [u8; 256],
}

impl UdpClient {
    fn start(io: &IoService) {
        let cl = Strand::new(io, UdpClient {
            soc: UdpSocket::new(Udp::v4()).unwrap(),
            timer: SteadyTimer::new(),
            buf: [0; 256],
        });
        SteadyTimer::async_wait_for(|cl| &cl.timer, &Duration::milliseconds(1), Self::on_wait, &cl);
        UdpSocket::async_recv(|cl| (&cl.soc, &mut cl.buf), 0, Self::on_receive, &cl);
    }

    fn on_wait(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            UdpSocket::cancel(|cl| &cl.soc, &cl);
        } else {
            panic!();
        }
    }

    fn on_receive(_: Strand<Self>, res: io::Result<usize>) {
        if let Err(err) = res {
            assert_eq!(err.kind(), io::ErrorKind::Other);  // cancel
        } else {
            panic!();
        }
    }
}

#[test]
fn main() {
    let io = IoService::new();
    UdpClient::start(&io);
    io.run();
}
