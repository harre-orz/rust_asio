use asyncio::IoContext;
use asyncio::ip::{TcpResolver, TcpSocket};
use std::env::args;
use std::process::exit;
use std::str;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn client(ctx: IoContext, host: String) -> Result {
    // Constructs a TcpResolver for IP version 4.
    let res = TcpResolver::v4(&ctx);
    // It connects resolved endpoints.
    let eps = res.resolve((host, "daytime"))?;
    let soc = TcpSocket::new(&ctx).connect(&eps)?;
    println!("connected to {:?}", soc.remote_endpoint().unwrap());
    // A server is send message to out program.
    let mut buf = [0; 256];
    let len = soc.read_some(&mut buf)?;
    println!("{}", str::from_utf8(&buf[..len])?);
    Ok(())
}

fn main() -> Result {
    let host: String = args().nth(1).unwrap_or_else(|| {
        println!("usage: client <host>");
        exit(1);
    });
    let ctx = IoContext::new()?;
    client(ctx, host)
}
