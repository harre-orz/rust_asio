use std::io;
use libc;
use std::os::unix::io::RawFd;
use traits::Protocol;
use io_service::{FromRawFd, IoService};
use super::LocalProtocol;

/// Returns a pair of connected UNIX domain sockets.
///
/// # Example
///
/// ```
/// use std::thread;
/// use asyncio::{IoService, Stream};
/// use asyncio::local::{LocalStream, LocalStreamSocket, connect_pair};
///
/// const MESSAGE: &'static str = "hello";
///
/// let io = &IoService::new();
/// let (tx, rx): (LocalStreamSocket, LocalStreamSocket) = connect_pair(io, LocalStream).unwrap();
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
pub fn connect_pair<P, S>(io: &IoService, pro: P) -> io::Result<(S, S)>
    where P: LocalProtocol,
          S: FromRawFd<P>,
{
    let (s1, s2) = try!(socketpair(&pro));
    unsafe { Ok((S::from_raw_fd(io, pro.clone(), s1), S::from_raw_fd(io, pro, s2))) }
}

fn socketpair<P>(pro: &P) -> io::Result<(RawFd, RawFd)>
    where P: Protocol,
{
    let mut sv = [0; 2];
    libc_try!(libc::socketpair(pro.family_type(), pro.socket_type(), pro.protocol_type(), sv.as_mut_ptr()));
    Ok((sv[0], sv[1]))
}
