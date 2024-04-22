extern crate asyio;

use crate::asyio::ip::TcpResolver;
use crate::asyio::IoContext;

use std::env::args;
use std::process::exit;
use std::str;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host: String = args().nth(1).unwrap_or_else(|| {
        println!("usage: client <host>");
        exit(1);
    });

    let ctx = &IoContext::new()?;
    let res = TcpResolver::new(ctx);

    let it = res.resolve((host, "daytime"))?;
    let (soc, ep) = res.connect(it)?;
    println!("connected to {:?}", ep);

    let mut buf = [0; 256];
    let len = soc.read_some(&mut buf)?;
    println!("{}", str::from_utf8(&buf[..len])?);

    Ok(())
}
