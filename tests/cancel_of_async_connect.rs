extern crate asyncio;
use std::io;
use std::time::Duration;
use asyncio::*;
use asyncio::ip::*;

static mut goal_flag: bool = false;

struct TcpClient {
    soc: TcpSocket,
    timer: SteadyTimer,
}

impl TcpClient {
    fn start(io: &IoService) {
        let cl = Strand::new(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
            timer: SteadyTimer::new(io),
        });
        cl.timer.async_wait_for(Duration::new(1, 0), cl.wrap(Self::on_wait));
        cl.soc.async_connect(&TcpEndpoint::new(IpAddrV4::new(192,0,2,1), 12345), cl.wrap(Self::on_connect));
    }

    fn on_wait(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            cl.soc.cancel();
        } else {
            panic!()
        }
    }

    fn on_connect(_: Strand<Self>, res: io::Result<()>) {
        if let Err(err) = res {
            assert_eq!(err.kind(), io::ErrorKind::Other);  // Cancel
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
