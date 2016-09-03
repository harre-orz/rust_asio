extern crate asio;
extern crate time;
use std::io;
use std::sync::Arc;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;
use time::Duration;

static mut goal_flag: bool = false;

fn start(io: &IoService) {
    let acc = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
    acc.set_option(ReuseAddr::new(true)).unwrap();
    acc.bind(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
    acc.listen().unwrap();
    acc.async_accept(bind(on_accept, &acc));
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
        let sv = Strand::new(io, TcpServer {
            _soc: soc,
            timer: SteadyTimer::new(io),
        });
        sv.timer.async_wait_for(Duration::milliseconds(1000), sv.wrap(Self::on_wait));
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
        let mut cl = Strand::new(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
            timer: SteadyTimer::new(io),
            buf: Vec::with_capacity(1024*1024),
        });
        unsafe {
            let len = cl.buf.capacity();
            cl.buf.set_len(len);
        }
        let ep = TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345);
        cl.soc.async_connect(&ep, cl.wrap(Self::on_connect));
    }

    fn on_connect(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            cl.timer.async_wait_for(Duration::milliseconds(500), cl.wrap(Self::on_wait));
            cl.soc.async_send(cl.buf.as_slice(), 0, cl.wrap(Self::on_send));
        } else {
            panic!();
        }
    }

    fn on_wait(cl: Strand<Self>, _: io::Result<()>) {
        cl.soc.cancel();
    }

    fn on_send(cl: Strand<Self>, res: io::Result<usize>) {
        match res {
            Ok(_) =>
                cl.soc.async_send(cl.buf.as_slice(), 0, cl.wrap(Self::on_send)),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::Other);  // Cancel
                unsafe { goal_flag = true; }
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
    assert!(unsafe { goal_flag })
}
