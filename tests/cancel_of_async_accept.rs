extern crate asyncio;
use std::io;
use std::time::Duration;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

static mut GOAL_FLAG: bool = false;

struct TcpAcceptor {
    soc: TcpListener,
    timer: SteadyTimer,
}

impl TcpAcceptor {
    fn start(io: &IoService) {
        IoService::strand(io, TcpAcceptor {
            soc: TcpListener::new(io, Tcp::v6()).unwrap(),
            timer: SteadyTimer::new(io),
        }, Self::on_start);
    }

    fn on_start(acc: Strand<Self>) {
        acc.soc.set_option(ReuseAddr::new(true)).unwrap();
        acc.soc.bind(&TcpEndpoint::new(IpAddrV6::any(), 12345)).unwrap();
        acc.soc.listen().unwrap();
        acc.timer.async_wait_for(Duration::new(1, 0), acc.wrap(Self::on_wait));
        acc.soc.async_accept(acc.wrap(Self::on_accept));
    }

    fn on_wait(acc: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            acc.soc.cancel();
        } else {
            panic!();
        }
    }

    fn on_accept(_: Strand<Self>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
        if let Err(err) = res {
            assert_eq!(err.kind(), io::ErrorKind::Other);  // cancel
            unsafe { GOAL_FLAG = true; }
        } else {
            panic!();
        }
    }
}

#[test]
fn main() {
    let io = IoService::new();
    TcpAcceptor::start(&io);
    io.run();
    assert!(unsafe { GOAL_FLAG });
}
