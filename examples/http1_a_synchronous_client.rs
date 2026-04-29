use asyncio::IoContext;
use asyncio::buffer::{IoStream, StreamBuf};
use asyncio::ip::{TcpResolver, TcpSocket};
use std::env::args;
use std::process::exit;
use std::str;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn http_client(ctx: &IoContext, host: String, path: String) -> Result {
    let mut sbuf = StreamBuf::from(format!("GET {} HTTP/1.1\r\nHost: {}\r\n\r\n", path, host));
    let res = TcpResolver::new(&ctx);
    let res = res.resolve((host, "http"))?;
    let soc = TcpSocket::new(&ctx).connect(&res)?;
    println!("connected to {:?}", soc.remote_endpoint().unwrap());

    let len = sbuf.len();
    let _ = soc.write_until(&mut sbuf, len)?;

    let len = soc.read_until(&mut sbuf, "\r\n")?;
    let buf = &sbuf.as_bytes()[..len];
    println!("len={}, buf={}", sbuf.len(), str::from_utf8(buf).unwrap());
    sbuf.consume(len);

    let len = soc.read_until(&mut sbuf, "\r\n\r\n")?;
    let buf = &sbuf.as_bytes()[..len];
    println!("len={}, buf={}", sbuf.len(), str::from_utf8(buf).unwrap());
    sbuf.consume(len);

    Ok(())
}

fn main() -> Result {
    let host: String = args().nth(1).unwrap_or_else(|| {
        println!("usage: http <host> <path>");
        exit(1);
    });
    let path: String = args().nth(2).unwrap_or_else(|| {
        println!("usage: http <host> <path>");
        exit(1);
    });
    let ctx = IoContext::new()?;
    http_client(&ctx, host, path)
}
