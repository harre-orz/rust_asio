extern crate asyncio;

use std::io;
use std::sync::Arc;
use std::time::Duration;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

static mut GOAL_FLAG: bool = false;

fn start(io: &IoService) {
    let acc = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
    acc.set_option(ReuseAddr::new(true)).unwrap();
    acc.bind(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
    acc.listen().unwrap();
    acc.async_accept(wrap(on_accept, &acc));
}

fn on_accept(_: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((soc, _)) = res {
        TcpServer::start(soc);
    } else {
        panic!();
    }
}

struct TcpServer {
    _soc: TcpSocket,
    timer: SteadyTimer,
}

impl TcpServer {
    fn start(soc: TcpSocket) {
        let io = &soc.io_service().clone();
        let sv = IoService::strand(io, TcpServer {
            _soc: soc,
            timer: SteadyTimer::new(io),
        });
        sv.dispatch(Self::on_start);
    }

    fn on_start(sv: Strand<Self>) {
        sv.timer.async_wait_for(Duration::new(1, 0), sv.wrap(Self::on_wait));
    }

    fn on_wait(_: Strand<Self>, _: io::Result<()>) {
    }
}

struct TcpClient {
    soc: TcpSocket,
    timer: SteadyTimer,
    buf: Vec<u8>,
}

impl TcpClient {
    fn start(io: &IoService) {
        let cl = IoService::strand(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
            timer: SteadyTimer::new(io),
            buf: Vec::with_capacity(1024*1024),
        });
        cl.dispatch(Self::on_start);
    }

    fn on_start(cl: Strand<Self>) {
        let len = cl.buf.capacity();
        unsafe { cl.get().buf.set_len(len); }
        let ep = TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345);
        cl.soc.async_connect(&ep, cl.wrap(Self::on_connect));
    }

    fn on_connect(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            println!("cl connected");
            cl.timer.async_wait_for(Duration::new(0, 500000000), cl.wrap(Self::on_wait));
            cl.soc.async_send(cl.buf.as_slice(), 0, cl.wrap(Self::on_send));
        } else {
            panic!();
        }
    }

    fn on_wait(cl: Strand<Self>, _: io::Result<()>) {
        println!("cl canceled");
        cl.soc.cancel();
    }

    fn on_send(cl: Strand<Self>, res: io::Result<usize>) {
        match res {
            Ok(_) => {
                println!("cl sent");
                cl.soc.async_send(cl.buf.as_slice(), 0, cl.wrap(Self::on_send));
            },
            Err(err) => {
                println!("cl failed to sent");
                assert_eq!(err.kind(), io::ErrorKind::Other);  // Cancel
                unsafe { GOAL_FLAG = true; }
            }
        }
    }
}

#[test]
fn main() {
    let io = IoService::new();
    start(&io);
    TcpClient::start(&io);
    io.run();
    assert!(unsafe { GOAL_FLAG })
}
