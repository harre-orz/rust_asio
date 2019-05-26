extern crate asyncio;
extern crate time;

use asyncio::*;
use asyncio::ip::*;
use asyncio::socket_base::*;

fn main() {
    let ctx = &IoContext::new().unwrap();

    // Constructs a TcpListener socket for IP version 4.
    let soc = TcpListener::new(ctx, Tcp::v4()).unwrap();

    // It sets a ReuseAddr socket option.
    soc.set_option(ReuseAddr::new(true)).unwrap();

    // It binds a TCP port 13.
    soc.bind(&TcpEndpoint::new(IpAddrV4::any(), 13)).unwrap();

    // It initializes to listen.
    soc.listen().unwrap();

    // It waits for accepted by a client connection.
    while let Ok((acc, ep)) = soc.accept() {
        let acc: TcpSocket = acc;
        println!("connected from {}", ep);

        // A client is accessing our program.
        // Makes the current time and transfer to the client.
        let buf = format!("{}\r\n", time::now().ctime());
        acc.write_some(buf.as_bytes()).unwrap();
    }
}
