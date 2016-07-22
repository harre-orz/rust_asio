extern crate asio;
extern crate time;

use std::io;
use asio::*;
use asio::ip::*;
use asio::socket_base::*;

struct DaytimeTcp {
    soc: TcpSocket,
    buf: String,
}

impl DaytimeTcp {
    fn start(io: &IoService, soc: TcpSocket) {
        // Constructs a Strand wrapped TcpSocket object and buffer to transfer to the client.
        let daytime = Strand::new(io, DaytimeTcp {
            soc: soc,
            buf: format!("{}\r\n", time::now().ctime())
        });

        unsafe { daytime.soc.async_write_some(ConstBuffer::new(daytime.buf.as_bytes()), Self::on_send, &daytime); }
    }

    fn on_send(_: Strand<Self>, _: io::Result<usize>) {
    }
}

fn on_accept(sv: Strand<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if let Ok((soc, ep)) = res {
        println!("connected from {:?}", ep);

        // Constructs a Daytime object.
        DaytimeTcp::start(sv.io_service(), soc);

        // It resets asynchronous accept operation.
        unsafe { sv.async_accept(on_accept, &sv); }
    }
}

fn main() {
    let io = &IoService::new();

    // Constructs a Strand wrapped TcpListener socket for IP version 4.
    let sv = Strand::new(io, TcpListener::new(io, Tcp::v4()).unwrap());

    // It sets a ReuseAddr socket option.
    sv.set_option(ReuseAddr::new(true)).unwrap();

    // It binds a TCP port 13.
    sv.bind(&TcpEndpoint::new(IpAddrV4::any(), 13)).unwrap();

    // It initializes to listen.
    sv.listen().unwrap();

    // It sets asynchronous accept operation.
    unsafe { sv.async_accept(on_accept, &sv); }

    // Runs aynchronous operations.
    io.run();
}
