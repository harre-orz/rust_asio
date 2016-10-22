extern crate asyncio;

use std::io;
use std::time::Duration;
use asyncio::*;
use asyncio::ip::*;

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
        cl.timer.async_wait_for(Duration::new(0, 1000000000), cl.wrap(Self::on_wait));
        cl.soc.async_receive(unsafe { &mut cl.get().buf }, 0, cl.wrap(Self::on_receive));
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
