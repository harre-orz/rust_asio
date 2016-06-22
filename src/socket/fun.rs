use std::io;
use std::cmp;
use Strand;
use socket::*;

// pub fn read_until<S: ReadWrite, C: MatchCondition>(soc: &S, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
//     let mut cur = 0;
//     loop {
//         match cond.is_match(&sbuf.as_slice()[cur..]) {
//             Ok(len) => return Ok(cur + len),
//             Err(len) => {
//                 cur = cmp::min(cur+len, sbuf.len());
//                 let len = try!(soc.read_some(try!(sbuf.prepare(4096))));
//                 sbuf.commit(len);
//             },
//         }
//     }
// }

// pub fn write_until<S: ReadWrite, C: MatchCondition>(soc: &S, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
//     let len = {
//         let len = match cond.is_match(sbuf.as_slice()) {
//             Ok(len) => len,
//             Err(len) => len,
//         };
//         try!(soc.write_some(&sbuf.as_slice()[..cmp::min(len, sbuf.len())]))
//     };
//     sbuf.consume(len);
//     Ok(len)
// }

pub fn async_connect<S, A, F, T>(a: A, ep: &S::Endpoint, callback: F, obj: &Strand<T>)
    where S: SocketConnector,
          A: Fn(&T) -> &S + Send,
          F: FnOnce(Strand<T>, io::Result<()>) + Send {
    S::async_connect(a, ep, callback, obj);
}

pub fn async_accept<S, A, F, T>(a: A, callback: F, obj: &Strand<T>)
    where S: SocketListener,
          A: Fn(&T) -> &S + Send,
          F: FnOnce(Strand<T>, io::Result<(S::Socket, S::Endpoint)>) + Send {
    S::async_accept(a, callback, obj);
}

pub fn async_recv<S, A, F, T>(a: A, callback: F, obj: &Strand<T>)
    where S: SendRecv,
          A: Fn(&mut T) -> (&S, &mut [u8]) + Send,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send {
    S::async_recv(a, 0, callback, obj)
}

pub fn async_recv_from<S, A, F, T>(a: A, callback: F, obj: &Strand<T>)
    where S: SendToRecvFrom,
          A: Fn(&mut T) -> (&S, &mut [u8]) + Send,
          F: FnOnce(Strand<T>, io::Result<(usize, S::Endpoint)>) + Send {
    S::async_recv_from(a, 0, callback, obj)
}

pub fn async_send<S, A, F, T>(a: A, callback: F, obj: &Strand<T>)
    where S: SendRecv,
          A: Fn(&T) -> (&S, &[u8]) + Send,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send {
    S::async_send(a, 0, callback, obj)
}

pub fn async_send_to<S, A, F, T>(a: A, ep: &S::Endpoint, callback: F, obj: &Strand<T>)
    where S: SendToRecvFrom,
          A: Fn(&T) -> (&S, &[u8]) + Send,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send {
    S::async_send_to(a, 0, ep, callback, obj)
}

pub fn async_read_until<S, A, C, F, T>(a: A, cond: C, callback: F, obj: &Strand<T>)
    where S: ReadWrite,
          A: Fn(&mut T) -> (&S, &mut StreamBuf) + Send + 'static,
          C: MatchCondition + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    S::async_read_until(a, cond, callback, obj)
}

pub fn async_write_until<S, A, C, F, T>(a: A, cond: C, callback: F, obj: &Strand<T>)
    where S: ReadWrite,
          A: Fn(&mut T) -> (&S, &mut StreamBuf) + Send + 'static,
          C: MatchCondition + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    S::async_write_until(a, cond, callback, obj)
}
