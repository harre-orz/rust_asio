extern crate asyncio;
use std::io;
use std::sync::Arc;
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
    if let Ok((_, _)) = res {
    } else {
        panic!();
    }
}

struct TcpClient {
    soc: TcpSocket,
    buf: Vec<u8>,
}

impl TcpClient {
    fn start(io: &IoService) {
        let cl = IoService::strand(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
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
            cl.soc.async_send(cl.buf.as_slice(), 0, cl.wrap(Self::on_send));
        } else {
            panic!();
        }
    }

    fn on_send(cl: Strand<Self>, res: io::Result<usize>) {
        match res {
            Ok(_) =>
                cl.soc.async_send(cl.buf.as_slice(), 0, cl.wrap(Self::on_send)),
            Err(err) => {
                assert!(err.kind() == io::ErrorKind::BrokenPipe || err.kind() == io::ErrorKind::Other);
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
    assert!(unsafe { GOAL_FLAG });
}
