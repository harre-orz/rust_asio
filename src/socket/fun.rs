use std::io;
use {IoService, Strand, Cancel};
use socket::*;

pub fn read_until<S: ReadWrite, C: MatchCondition>(soc: &S, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
    let mut cur = 0;
    loop {
        match cond.is_match(&sbuf.as_slice()[cur..]) {
            Ok(len) => return Ok(cur + len),
            Err(len) => {
                cur += len;
                let len = try!(soc.read_some(try!(sbuf.prepare(4096))));
                sbuf.commit(len);
            },
        }
    }
}

pub fn write_until<S: ReadWrite, C: MatchCondition>(soc: &S, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
    let len = {
        let len = match cond.is_match(sbuf.as_slice()) {
            Ok(len) => len,
            Err(len) => len,
        };
        try!(soc.write_some(&sbuf.as_slice()[..len]))
    };
    sbuf.consume(len);
    Ok(len)
}

pub fn async_connect<S, A, F, T>(a: A, ep: &S::Endpoint, callback: F, obj: &Strand<T>)
    where S: SocketConnector,
          A: FnOnce(&T) -> &S + Send,
          F: FnOnce(Strand<T>, io::Result<()>) + Send,
{
    S::async_connect(a, ep, callback, obj);
}

pub fn async_accept<S, A, F, T>(a: A, callback: F, obj: &Strand<T>)
    where S: SocketListener,
          A: FnOnce(&T) -> &S + Send,
          F: FnOnce(Strand<T>, io::Result<(S::Socket, S::Endpoint)>) + Send,
{
    S::async_accept(a, callback, obj);
}

pub fn async_recv<S, A, F, T>(a: A, callback: F, obj: &Strand<T>)
    where S: SendRecv,
          A: FnOnce(&mut T) -> (&S, &mut [u8]) + Send,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send,
{
    S::async_recv(a, 0, callback, obj)
}

pub fn async_recv_from<S, A, F, T>(a: A, callback: F, obj: &Strand<T>)
    where S: SendToRecvFrom,
          A: FnOnce(&mut T) -> (&S, &mut [u8]) + Send,
          F: FnOnce(Strand<T>, io::Result<(usize, S::Endpoint)>) + Send,
{
    S::async_recv_from(a, 0, callback, obj)
}

pub fn async_send<S, A, F, T>(a: A, callback: F, obj: &Strand<T>)
    where S: SendRecv,
          A: FnOnce(&T) -> (&S, &[u8]) + Send,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send,
{
    S::async_send(a, 0, callback, obj)
}

pub fn async_send_to<S, A, F, T>(a: A, ep: &S::Endpoint, callback: F, obj: &Strand<T>)
    where S: SendToRecvFrom,
          A: FnOnce(&T) -> (&S, &[u8]) + Send,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send,
{
    S::async_send_to(a, 0, ep, callback, obj)
}

pub fn async_read_some<S, A, F, T>(a: A, callback: F, obj: &Strand<T>)
    where S: ReadWrite,
          A: FnOnce(&mut T) -> (&S, &mut [u8]) + Send,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send,
{
    S::async_read_some(a, callback, obj)
}

pub fn async_read_until<S, A, C, F, T>(a: A, sbuf: &mut StreamBuf, cond: C, callback: F, obj: &Strand<T>)
    where S: ReadWrite,
          A: FnOnce(&mut T) -> (&S, &mut StreamBuf) + Send,
          C: MatchCondition + Clone + Send,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send,
{
    S::async_read_until(a, cond, callback, obj)
}

pub fn async_write_some<S, A, F, T>(a: A, callback: F, obj: &Strand<T>)
    where S: ReadWrite,
          A: FnOnce(&T) -> (&S, &[u8]) + Send,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send,
{
    S::async_write_some(a, callback, obj)
}

pub fn async_write_until<S, A, C, F, T>(a: A, sbuf: &mut StreamBuf, cond: C, callback: F, obj: &Strand<T>)
    where S: ReadWrite,
          A: FnOnce(&mut T) -> (&S, &mut StreamBuf) + Send,
          C: MatchCondition + Clone + Send,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send,
{
    S::async_write_until(a, cond, callback, obj)
}

pub fn cancel<C, A, T>(a: A, obj: &Strand<T>)
    where C: Cancel,
          A: FnOnce(&T) -> &C,
{
    C::cancel(a, obj)
}
