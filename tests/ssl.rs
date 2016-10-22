extern crate asyncio;
use std::thread;
use asyncio::*;
use asyncio::ip::*;
use asyncio::ssl::*;
use asyncio::socket_base::*;

#[test]
fn main() {
    let io = IoService::new();
    let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);

    let soc = TcpListener::new(&io, ep.protocol()).unwrap();
    soc.set_option(ReuseAddr::new(true)).unwrap();
    soc.bind(&ep).unwrap();
    soc.listen().unwrap();

    let t1 = thread::spawn(move || {
        let (sv, ep) = soc.accept().unwrap();
        println!("accepted from {}", ep);

        let ssl = SslContext::sslv23();
        ssl.use_certificate_chain_file("keys/server.crt", FileFormat::PEM).unwrap();
        ssl.use_private_key_file("keys/server.key", FileFormat::PEM).unwrap();

        let soc = SslStream::new(sv, &ssl).unwrap();
        soc.handshake(Handshake::Server).unwrap();

        let mut buf = [0; 1024];
        let len = soc.read_some(&mut buf).unwrap();
        assert_eq!(&buf[..len], "hello".as_bytes());

    });

    let t2 = thread::spawn(move || {
        let cl = TcpSocket::new(&io, ep.protocol()).unwrap();
        cl.connect(&ep).unwrap();
        println!("connected to {}", ep);

        let ssl = SslContext::sslv23();
        ssl.load_verify_file("keys/server.crt").unwrap();

        let soc = SslStream::new(cl, &ssl).unwrap();
        soc.handshake(Handshake::Client).unwrap();

        soc.write_some("hello".as_bytes()).unwrap();
    });

    t1.join().unwrap();
    t2.join().unwrap();
}
