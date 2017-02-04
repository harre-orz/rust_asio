extern crate asyncio;
use std::io;
use std::sync::{Arc, Mutex};
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

const MESSAGE: &'static str = "hello world";

static mut GOAL_FLAG: bool = false;

fn start(ctx: &IoContext) {
    let acc = TcpListener::new(ctx, Tcp::v4()).unwrap();
    acc.set_option(ReuseAddr::new(true)).unwrap();
    acc.bind(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
    acc.listen().unwrap();
    let acc = Arc::new(Mutex::new(acc));
    acc.lock().unwrap().async_accept(wrap(on_accept, &acc));
}

fn on_accept(acc: Arc<Mutex<TcpListener>>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((soc, _)) = res {
        println!("sv accepted");
        TcpServer::start(acc.lock().unwrap().as_ctx(), soc);
    } else {
        panic!("{:?}", res);
    }
}

struct TcpServer {
    soc: TcpSocket,
    buf: [u8; 256],
}

impl TcpServer {
    fn start(ctx: &IoContext, soc: TcpSocket) {
        let sv = IoContext::strand(ctx, TcpServer {
            soc: soc,
            buf: [0; 256],
        });
        sv.dispatch(Self::on_start);
    }

    fn on_start(sv: Strand<Self>) {
        sv.soc.async_read_some(&mut sv.get().buf, sv.wrap(Self::on_recv));
    }

    fn on_recv(sv: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            println!("sv received {}", len);
            assert_eq!(len, MESSAGE.len());
            sv.soc.async_write_some(&sv.buf[..MESSAGE.len()], sv.wrap(Self::on_send));
        } else {
            panic!("{:?}", res);
        }
    }

    fn on_send(sv: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            println!("sv sent {}", len);
            assert_eq!(len, MESSAGE.len());
            sv.soc.async_read_some(&mut sv.get().buf, sv.wrap(Self::on_fin));
        } else {
            panic!("{:?}", res);
        }
    }

    fn on_fin(_: Strand<Self>, res: io::Result<usize>) {
        println!("res {:?}", res);
        if let Ok(len) = res {
            println!("sv fin {}", len);
            assert_eq!(len, MESSAGE.len());
        } else {
            panic!("{:?}", res);
        }
    }
}


struct TcpClient {
    soc: TcpSocket,
    buf: [u8; 256],
}

impl TcpClient {
    fn start(ctx: &IoContext) {
        let cl = IoContext::strand(ctx, TcpClient {
            soc: TcpSocket::new(ctx, Tcp::v4()).unwrap(),
            buf: [0; 256],
        });
        cl.dispatch(Self::on_start);
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
            panic!("{:?}", res);
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
            panic!("{:?}", res);
        }
    }

    fn on_fin(_: Strand<Self>, res: io::Result<usize>) {
        if let Ok(len) = res {
            println!("cl fin {}", len);
            assert_eq!(len, MESSAGE.len());
            unsafe { GOAL_FLAG = true; }
        } else {
            panic!("{:?}", res);
        }
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    start(ctx);
    TcpClient::start(ctx);
    ctx.run();
    println!("goaled");
    assert!(unsafe { GOAL_FLAG });
}
