extern crate asio;
use std::io;
use std::sync::Arc;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;

static mut goal_flag: bool = false;

fn start(io: &IoService) {
    let acc = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
    acc.set_option(ReuseAddr::new(true)).unwrap();
    acc.bind(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
    acc.listen().unwrap();
    acc.async_accept(bind(on_accept, &acc));
}

fn on_accept(_: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    let (soc, _) = res.unwrap();
    TcpServer::start(soc);
}

struct TcpServer {
    soc: TcpSocket,
    buf: [u8; 1],
}

impl TcpServer {
    fn start(soc: TcpSocket) {
        let mut vec = Vec::new();
        vec.push('\r' as u8);
        vec.push('\n' as u8);
        for _ in 0..10000 {
            vec.push(0x30);
        }
        vec.push('\r' as u8);
        vec.push('\n' as u8);
        soc.write_some(vec.as_slice()).unwrap();

        let io = &soc.io_service().clone();
        let sv = Strand::new(io, TcpServer {
            soc: soc,
            buf: [0],
        });

        sv.soc.async_read_some(unsafe { &mut sv.get().buf }, sv.wrap(Self::on_read));
    }

    fn on_read(_: Strand<Self>, res: io::Result<usize>) {
        assert!(res.is_err());
    }
}

struct TcpClient {
    soc: TcpSocket,
    buf: StreamBuf,
}

impl TcpClient {
    fn start(io: &IoService) {
        let cl = Strand::new(io, TcpClient {
            soc: TcpSocket::new(io, Tcp::v4()).unwrap(),
            buf: StreamBuf::new(65536),
        });
        cl.soc.connect(&TcpEndpoint::new(IpAddrV4::new(127,0,0,1), 12345)).unwrap();
        async_read_until(&cl.soc, unsafe { &mut cl.get().buf }, "\r\n", cl.wrap(Self::on_read1));
    }

    fn on_read1(cl: Strand<Self>, res: io::Result<usize>) {
        let size = res.unwrap();
        assert_eq!(size, 2);
        async_read_until(&cl.soc, unsafe { &mut cl.get().buf }, "\r\n", cl.wrap(Self::on_read2));
    }

    fn on_read2(mut cl: Strand<Self>, res: io::Result<usize>) {
        let size = res.unwrap();
        assert_eq!(size, 2);
        cl.buf.consume(2);
        async_read_until(&cl.soc, unsafe { &mut cl.get().buf }, "\r\n", cl.wrap(Self::on_read3));
    }

    fn on_read3(_: Strand<Self>, res: io::Result<usize>) {
        let size = res.unwrap();
        assert_eq!(size, 10002);
        unsafe { goal_flag = true; }
    }
}

#[test]
fn main() {
    let io = &IoService::new();
    start(io);
    TcpClient::start(io);
    io.run();
    assert!(unsafe { goal_flag });
}
