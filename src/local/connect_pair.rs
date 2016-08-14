use std::io;
use {IoObject, FromRawFd};
use super::LocalProtocol;
use backbone::{socketpair};

pub fn connect_pair<T: IoObject, P: LocalProtocol, S: FromRawFd<P>>(io: &T, pro: P) -> io::Result<(S, S)> {
    let (s1, s2) = try!(socketpair(&pro));
    Ok(unsafe { (S::from_raw_fd(io, pro.clone(), s1), S::from_raw_fd(io, pro, s2)) })
}
