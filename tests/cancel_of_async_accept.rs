extern crate asio;
extern crate time;
use std::io;
use time::Duration;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;

static mut goal_flag: bool = false;

struct TcpAcceptor {
    soc: TcpListener,
    timer: SteadyTimer,
}

impl TcpAcceptor {
    fn start(io: &IoService) {
        let acc = Strand::new(io, TcpAcceptor {
            soc: TcpListener::new(io, Tcp::v6()).unwrap(),
            timer: SteadyTimer::new(io),
        });
        acc.soc.set_option(&ReuseAddr::on()).unwrap();
        acc.soc.bind(&TcpEndpoint::new(IpAddrV6::any(), 12345)).unwrap();
        acc.soc.listen().unwrap();
        acc.timer.async_wait_for(&Duration::milliseconds(1), Self::on_wait, &acc);
        acc.soc.async_accept(Self::on_accept, &acc);
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
    io.run();
    assert!(unsafe { goal_flag });
}
