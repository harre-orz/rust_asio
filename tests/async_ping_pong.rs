extern crate asyncio;
use std::io;
use std::sync::Arc;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

const MESSAGE: &'static str = "hello world";

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
        println!("sv accepted");
        TcpServer::start(soc);
    } else {
        panic!();
    }
}

struct TcpServer {
    soc: TcpSocket,
    buf: [u8; 256],
}

impl TcpServer {
    fn start(soc: TcpSocket) {
        let io = &soc.io_service().clone();
        IoService::strand(io, TcpServer {
            soc: soc,
            buf: [0; 256],
        }, Self::on_start);
    }

    fn on_start(sv: Strand<Self>) {
        sv.soc.async_read_some(&mut sv.get().buf, sv.wrap(Self::on_recv1));
    }

    fn on_recv1(sv: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            println!("sv received-1 {}", len);
            assert_eq!(len, MESSAGE.len());
            sv.soc.async_write_some(&sv.buf[..MESSAGE.len()], sv.wrap(Self::on_send1));
        } else {
            panic!();
        }
    }

    fn on_send1(sv: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            println!("sv sent-1 {}", len);
            assert_eq!(len, MESSAGE.len());
            sv.soc.async_read_some(&mut sv.get().buf, sv.wrap(Self::on_recv2));
        } else {
            panic!();
        }
    }

    fn on_recv2(_: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            println!("sv received-2 {}", len);
            assert_eq!(len, MESSAGE.len());
        } else {
            panic!();
        }
    }
}

struct TcpClient {
    soc: TcpSocket,
    buf: [u8; 256],
}

impl TcpClient {
    fn start(io: &IoService) {
        IoService::strand(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
            buf: [0; 256],
        }, Self::on_start);
    }

    fn on_start(cl: Strand<Self>) {
        println!("cl start");
        let ep = TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345);
        cl.soc.async_connect(&ep, cl.wrap(Self::on_connect));
    }

    fn on_connect(cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            println!("cl connected");
            cl.soc.async_write_some(MESSAGE.as_bytes(), cl.wrap(Self::on_send));
        } else {
            panic!();
        }
    }

    fn on_send(cl: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            println!("cl sent {}", len);
            assert_eq!(len, MESSAGE.len());
            cl.soc.async_read_some(&mut cl.get().buf, cl.wrap(Self::on_recv));
        } else {
            panic!();
        }
    }

    fn on_recv(cl: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            println!("cl received {}", len);
            assert_eq!(len, MESSAGE.len());
            cl.soc.async_write_some(MESSAGE.as_bytes(), cl.wrap(Self::on_fin));
        } else {
            panic!();
        }
    }

    fn on_fin(_: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            println!("cl sent {}", len);
            assert_eq!(len, MESSAGE.len());
            unsafe { GOAL_FLAG = true; }
        } else {
            panic!();
        }
    }
}

#[test]
fn main() {
    let io = &IoService::new();
    start(io);
    TcpClient::start(io);
    io.run();
    assert!(unsafe { GOAL_FLAG });
}
