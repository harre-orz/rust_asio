extern crate asio;
extern crate time;
use std::io;
use time::Duration;
use asio::*;
use asio::ip::*;

static mut goal_flag: bool = false;

struct UdpClient {
    soc: UdpSocket,
    timer: SteadyTimer,
    buf: [u8; 256],
}

impl UdpClient {
    fn start(io: &IoService) {
        let cl = Strand::new(io, UdpClient {
            soc: UdpSocket::new(io, Udp::v4()).unwrap(),
            timer: SteadyTimer::new(io),
            buf: [0; 256],
        });
        cl.timer.async_wait_for(&Duration::milliseconds(1), Self::on_wait, &cl);
        cl.soc.async_receive(|cl| &mut cl.buf, 0, Self::on_receive, &cl);
    }

    fn on_wait(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            cl.soc.cancel();
        } else {
            panic!();
        }
    }

    fn on_receive(_: Strand<Self>, res: io::Result<usize>) {
        if let Err(err) = res {
            assert_eq!(err.kind(), io::ErrorKind::Other);  // cancel
            unsafe { goal_flag = true; }
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
    assert!(unsafe { goal_flag })
}
