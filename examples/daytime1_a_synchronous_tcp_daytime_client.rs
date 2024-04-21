extern crate asyio;

use crate::asyio::ip::TcpResolver;
use crate::asyio::{IoContext, YieldContext};

use std::env::args;
use std::process::exit;
use std::str;

fn daytime_client(ctx: &mut YieldContext, host: String) -> Result<(), Box<dyn std::error::Error>> {
    let res = TcpResolver::new(ctx.as_ctx());

    let it = res.resolve((host, "daytime"))?;
    let (soc, ep) = res
        .connect(it, ctx)?;
    println!("connected to {:?}", ep);

    let mut buf = [0; 256];
    let len = soc.read_some(&mut buf, ctx)?;
    println!("{}", str::from_utf8(&buf[..len])?);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host: String = args().nth(1).unwrap_or_else(|| {
        println!("usage: client <host>");
        exit(1);
    });

    let ctx = &IoContext::new()?;
    ctx.spawn(|ctx| {
        let _ = daytime_client(ctx, host);
    })?;
    ctx.run();

    Ok(())
}
