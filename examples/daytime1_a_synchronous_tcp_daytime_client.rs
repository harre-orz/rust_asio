use asyio::ip::TcpResolver;
use asyio::IoContext;
use std::env::args;
use std::process::exit;
use std::str;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host: String = args().nth(1).unwrap_or_else(|| {
        println!("usage: client <host>");
        exit(1);
    });
    let ctx = IoContext::new()?;
    // Constructs a TcpResolver for IP version 4.
    let res = TcpResolver::v4(&ctx);
    // It connects resolved endpoints.
    let (soc, ep) = res.connect((host, "daytime"))?;
    println!("connected to {:?}", ep);
    // A server is send message to out program.
    let mut buf = [0; 256];
    let len = soc.read_some(&mut buf)?;
    println!("{}", str::from_utf8(&buf[..len])?);
    Ok(())
}
