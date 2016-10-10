extern crate asyncio;

use std::io::{self, Write};
use std::env::args;
use std::process::exit;
use std::sync::Arc;
use std::str::from_utf8;
use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

struct HttpSession {
    soc: TcpSocket,
    buf: StreamBuf,
}

impl HttpSession {
    fn start(io: &IoService, soc: TcpSocket) {
        let http = Strand::new(io, HttpSession {
            soc: soc,
            buf: StreamBuf::new(65536),
        });

        async_read_until(&http.soc, &mut http.as_mut().buf, "\r\n", http.wrap(Self::on_request_line));
    }

    fn on_request_line(mut http: Strand<Self>, res: io::Result<usize>) {
        if let Ok(size) = res {
            println!("({}) request line: {:?}", size, from_utf8(&http.buf.as_slice()[..size-2]).unwrap());

            http.buf.consume(size);
            async_read_until(&http.soc, &mut http.as_mut().buf, "\r\n", http.wrap(Self::on_request_header));
        }
    }

    fn on_request_header(mut http: Strand<Self>, res: io::Result<usize>) {
        if let Ok(size) = res {
            if size > 2 {
                println!("({}) request header: {:?}", size, from_utf8(&http.buf.as_slice()[..size-2]).unwrap());

                http.buf.consume(size);
                async_read_until(&http.soc, &mut http.as_mut().buf, "\r\n", http.wrap(Self::on_request_header));
            } else {
                let len = http.buf.len();
                http.buf.consume(len);

                let len = http.buf.write("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-type: text/html\r\nContent-Length: 4\r\n\r\nhoge".as_bytes()).unwrap();
                async_write_until(&http.soc, &mut http.as_mut().buf, len, http.wrap(Self::on_response));
            }
        }
    }

    fn on_response(mut _http: Strand<Self>, res: io::Result<usize>) {
        if let Ok(size) = res {
            println!("({}) response", size);
        }
    }
}

fn on_accept(soc: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
    if soc.io_service().stopped() {
        return;
    }

    if let Ok((acc, ep)) = res {
        println!("connected from {}", ep);
        HttpSession::start(soc.io_service(), acc);
    }

    soc.async_accept(wrap(on_accept, &soc));
}

fn main() {
    let port = args().nth(1).unwrap_or_else(|| {
        println!("usage: http_server <port>");
        exit(1);
    });
    let port = u16::from_str_radix(&port, 10).unwrap();
    let ep = TcpEndpoint::new(IpAddrV6::any(), port);

    let io = &IoService::new();
    let soc = Arc::new(TcpListener::new(io, ep.protocol()).unwrap());
    soc.set_option(ReuseAddr::new(true)).unwrap();
    soc.bind(&ep).unwrap();
    soc.listen().unwrap();

    println!("start {}", ep);
    soc.async_accept(wrap(on_accept, &soc));

    io.run();
}
