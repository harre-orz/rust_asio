extern crate asyncio;

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

static mut GOAL_FLAG: bool = false;

struct TcpServer {
    _soc: TcpSocket,
    timer: SteadyTimer,
}

impl TcpServer {
    fn start(ctx: &IoContext, soc: TcpSocket) -> io::Result<()> {
        Ok(Strand::new(
            ctx,
            TcpServer {
                _soc: soc,
                timer: SteadyTimer::new(ctx),
            },
        ).dispatch(Self::on_start))
    }

    fn on_start(mut sv: Strand<Self>) {
        println!("sv do_dispatch");
        sv.timer.expires_from_now(Duration::new(1, 0));
        sv.timer.async_wait(sv.wrap(Self::on_wait));
    }

    fn on_wait(_: Strand<Self>, _: io::Result<()>) {
        println!("on_wait");
    }
}

fn on_accept(acc: Arc<Mutex<TcpListener>>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((soc, _)) = res {
        println!("on_accept");
        TcpServer::start(acc.lock().unwrap().as_ctx(), soc).unwrap();
    } else {
        panic!("{:?}", res);
    }
}

struct TcpClient {
    soc: TcpSocket,
    timer: SteadyTimer,
    buf: Vec<u8>,
}

impl TcpClient {
    fn start(ctx: &IoContext) -> io::Result<()> {
        let mut buf = Vec::with_capacity(1024 * 1024);
        let len = buf.capacity();
        unsafe {
            buf.set_len(len);
        }

        let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);
        Ok(Strand::new(
            ctx,
            TcpClient {
                soc: try!(TcpSocket::new(ctx, ep.protocol())),
                timer: SteadyTimer::new(ctx),
                buf: buf,
            },
        ).dispatch(move |cl| Self::on_dispatch(cl, ep)))
    }

    fn on_dispatch(cl: Strand<Self>, ep: TcpEndpoint) {
        println!("cl do_dispatch");
        cl.soc.async_connect(&ep, cl.wrap(Self::on_connect));
    }

    fn on_connect(mut cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            println!("cl connected");
            cl.timer.expires_from_now(Duration::new(0, 500000000));
            cl.timer.async_wait(cl.wrap(Self::on_wait));
            cl.soc
                .async_send(cl.buf.as_slice(), 0, cl.wrap(Self::on_send));
        } else {
            panic!("{:?}", res);
        }
    }

    fn on_send(cl: Strand<Self>, res: io::Result<usize>) {
        match res {
            Ok(_) => {
                println!("cl sent");
                cl.soc
                    .async_send(cl.buf.as_slice(), 0, cl.wrap(Self::on_send));
            }
            Err(err) => {
                println!("cl failed to sent");
                assert_eq!(err.kind(), io::ErrorKind::Other); // Cancel
                unsafe {
                    GOAL_FLAG = true;
                }
            }
        }
    }

    fn on_wait(mut cl: Strand<Self>, res: io::Result<()>) {
        if let Ok(_) = res {
            println!("cl canceled");
            cl.soc.cancel();
        } else {
            panic!("{:?}", res);
        }
    }
}

#[test]
fn main() {
    let ctx = &IoContext::new().unwrap();
    let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);
    let acc = TcpListener::new(ctx, ep.protocol()).unwrap();
    acc.set_option(ReuseAddr::new(true)).unwrap();
    acc.bind(&ep).unwrap();
    acc.listen().unwrap();
    let acc = Arc::new(Mutex::new(acc));
    acc.lock().unwrap().async_accept(wrap(on_accept, &acc));
    TcpClient::start(ctx).unwrap();
    ctx.run();
    assert!(unsafe { GOAL_FLAG })
}
