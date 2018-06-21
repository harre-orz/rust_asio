#[cfg(unix)]
mod posix;
#[cfg(unix)]
pub use self::posix::PosixFdSet as FdSet;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use self::win::WinFdSet as FdSet;

// #[test]
// fn test_fd_set_1() {
//     use ffi::{AsRawFd, INVALID_SOCKET};
//     use core::IoContext;
//     use ip::{IpProtocol, Tcp, TcpSocket};
//
//     let ctx = &IoContext::new().unwrap();
//     let mut fds = FdSet::new();
//     assert_eq!(fds.max_fd(), INVALID_SOCKET);
//
//
//     let soc = &TcpSocket::new(ctx, Tcp::v4()).unwrap();
//     fds.set(soc);
//     assert_eq!(fds.max_fd(), soc.as_raw_fd());
//     assert!(fds.is_set(soc));
// }
//
// #[test]
// fn test_fd_set_1000() {
//     use core::IoContext;
//     use ip::{IpProtocol, Tcp, TcpSocket};
//
//     let ctx = &IoContext::new().unwrap();
//     let mut fds = FdSet::new();
//     for _ in 0..1000 {
//         let soc = &TcpSocket::new(ctx, Tcp::v4()).unwrap();
//         fds.set(soc);
//     }
// }
