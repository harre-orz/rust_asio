extern crate asio;
extern crate time;
use std::io;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;
use time::Duration;

static mut goal_flag: bool = false;

struct TcpAcceptor {
    soc: TcpListener,
}

impl TcpAcceptor {
    fn start(io: &IoService) {
        let acc = Strand::new(io, TcpAcceptor {
            soc: TcpListener::new(io, Tcp::v4()).unwrap(),
        });
        acc.soc.set_option(ReuseAddr::new(true)).unwrap();
        acc.soc.bind(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
        acc.soc.listen().unwrap();
        unsafe { acc.soc.async_accept(Self::on_accept, &acc); }
    }

    fn on_accept(acc: Strand<Self>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
        if let Ok((soc, _)) = res {
            TcpServer::start(acc.io_service(), soc);
        } else {
            panic!();
        }
    }
}

struct TcpServer {
    _soc: TcpSocket,
    timer: SteadyTimer,
}

impl TcpServer {
    fn start(io: &IoService, soc: TcpSocket) {
        let sv = Strand::new(io, TcpServer {
            _soc: soc,
            timer: SteadyTimer::new(io),
        });
        unsafe { sv.timer.async_wait_for(&Duration::milliseconds(1000), Self::on_wait, &sv); }
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
        unsafe { cl.soc.async_connect(&ep, Self::on_connect, &cl); }
    }

    fn on_connect(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            unsafe {
                cl.timer.async_wait_for(&Duration::milliseconds(500), Self::on_wait, &cl);
                cl.soc.async_send(ConstBuffer::new(cl.buf.as_slice()), 0, Self::on_send, &cl);
            }
        } else {
            panic!();
        }
    }

    fn on_wait(cl: Strand<Self>, _: io::Result<()>) {
        cl.soc.cancel();
    }

    fn on_send(cl: Strand<Self>, res: io::Result<usize>) {
        match res {
            Ok(_) => unsafe {
                cl.soc.async_send(ConstBuffer::new(cl.buf.as_slice()), 0, Self::on_send, &cl);
            },
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
    TcpAcceptor::start(&io);
    TcpClient::start(&io);
    io.run();
    assert!(unsafe { goal_flag })
}
