
#[test]
fn test_msgbuf_as_endpoint_unchecked() {
    use crate::ip::UdpEndpoint;

    let mbuf = MsgBuf::new(1024).unwrap();
    assert_eq!(mbuf.len(), 0);
    assert_eq!(mbuf.max_len(), 1);
    unsafe {
        // returns indefinite, but overflow
        mbuf.as_endpoint_unchecked::<UdpEndpoint>();
    }
}

#[test]
fn test_msgbuf_prepare() {
    use crate::ip::UdpEndpoint;
    use std::net::Ipv4Addr;

    let ep = UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345);
    let mut mbuf = MsgBuf::new(1024).unwrap();

    let buf = mbuf.prepare().unwrap();
    assert_eq!(buf.len(), 1024);

    buf.commit(100, &ep);
    assert_eq!(mbuf.len(), 1);
    assert_eq!(mbuf.max_len(), 1);
    assert_eq!(mbuf.as_bytes().len(), 0);
}

#[test]
fn test_msgbuf_prepare_next() {
    use crate::ip::UdpEndpoint;
    use std::io::Write;
    use std::net::Ipv4Addr;

    let ep = UdpEndpoint::v4(Ipv4Addr::LOCALHOST, 12345);
    let mut mbuf = MsgBuf::new(1024).unwrap();

    let mut buf = mbuf.prepare().unwrap();
    let len = buf.as_bytes_mut().write(b"hello world").unwrap();
    buf.commit(len, &ep);
    assert_eq!(mbuf.as_bytes(), &[]);
    assert_eq!(mbuf.next().unwrap(), len);
    assert_eq!(&mbuf.as_bytes()[..len], b"hello world");
    assert_eq!(&unsafe { mbuf.as_endpoint_unchecked() }, &ep);
}