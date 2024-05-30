use asyio::ip::TcpResolver;
use asyio::IoContext;
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
    let (soc, ep) = res.async_connect((host, "daytime")).await?;
    println!("connected to {:?}", ep);

    // A server is send message to out program.
    let mut buf = [0; 256];
    let len = soc.async_read_some(&mut buf).await?;
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
