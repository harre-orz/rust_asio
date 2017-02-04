use ffi::socketpair;
use core::{IoContext, Socket};
use local::LocalProtocol;

use std::io;

/// Returns a pair of connected UNIX domain sockets.
///
/// # Example
///
/// ```
/// use std::thread;
/// use asyncio::{IoContext, Stream};
/// use asyncio::local::{LocalStream, LocalStreamSocket, connect_pair};
///
/// const MESSAGE: &'static str = "hello";
///
/// let ctx = &IoContext::new().unwrap();
/// let (tx, rx) = connect_pair(ctx, LocalStream).unwrap();
///
/// let thrd = thread::spawn(move|| {
///     let mut buf = [0; 32];
///     let len = rx.read_some(&mut buf).unwrap();
///     assert_eq!(len, MESSAGE.len());
///     assert_eq!(&buf[..len], MESSAGE.as_bytes());
/// });
///
/// tx.write_some(MESSAGE.as_bytes()).unwrap();
/// thrd.join().unwrap();
/// ```
pub fn connect_pair<P>(ctx: &IoContext, pro: P) -> io::Result<(P::Socket, P::Socket)>
    where P: LocalProtocol,
{
    let (s1, s2) = try!(socketpair(&pro));
    Ok((
        unsafe { P::Socket::from_raw_fd(ctx, pro.clone(), s1) },
        unsafe { P::Socket::from_raw_fd(ctx, pro, s2) }
    ))
}
