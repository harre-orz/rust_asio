use asyncio::IoContext;
use asyncio::ip::{TcpResolver, TcpSocket};
use futures::executor::LocalPool;
use futures::task::SpawnExt;
use std::env::args;
use std::process::exit;
use std::str;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

async fn client(ctx: IoContext, host: String) -> Result {
    // Constructs a TcpResolver for IP version 4.
    let res = TcpResolver::v4(&ctx);

    // It connects resolved endpoints.
    let eps = res.resolve((host, "daytime"))?;
    let soc = TcpSocket::new(&ctx).connect(eps).await?;
    println!("connected to {:?}", soc.remote_endpoint()?);

    // A server is send message to out program.
    let mut buf = [0; 256];
    let len = soc.read_some(&mut buf).await?;
    println!("{}", str::from_utf8(&buf[..len])?);

    Ok(())
}

fn main() -> Result {
    let host: String = args().nth(1).unwrap_or_else(|| {
        println!("usage: client <host>");
        exit(1);
    });
    let ctx = IoContext::new()?;
    let mut pool = LocalPool::new();
    {
        let ctx = ctx.clone();
        pool.spawner()
            .spawn(async move { client(ctx, host).await.unwrap() })?;
    }
    pool.spawner().spawn(async move {
        ctx.run().await.unwrap();
    })?;
    pool.run();
    Ok(())
}
