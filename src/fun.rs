use std::io;
use {IoObject, Strand, Protocol, NonBlocking, StreamSocket, StreamBuf, MatchCondition};
use {UnsafeThreadableCell};
use ops::async::*;

pub fn read_until<P: Protocol, C: MatchCondition>(soc: &StreamSocket<P>, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
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

pub fn write_until<P: Protocol, C: MatchCondition>(soc: &StreamSocket<P>, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
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

fn async_read_until_impl<S, C, F, T>(soc: &S, sbuf: &mut StreamBuf, mut cond: C, callback: F, strand: &Strand<T>, mut cur: usize)
    where S: AsIoActor + NonBlocking + Send + 'static,
          C: MatchCondition + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    match cond.is_match(&sbuf.as_slice()[cur..]) {
        Ok(len) => {
            soc.io_service().post_strand(
                move |strand| callback(strand, Ok(cur + len)), strand);
        },
        Err(len) => {
            cur += len;
            let ptr = sbuf as *mut StreamBuf;
            match sbuf.prepare(4096) {
                Ok(buf) => {
                    let mut ptr_ = UnsafeThreadableCell::new((soc as *const S, ptr));
                    async_read(soc, buf, move |strand, res| {
                        match res {
                            Ok(len) => {
                                let sbuf = unsafe { &mut *ptr_.1 };
                                sbuf.commit(len);
                                async_read_until_impl(unsafe { &*ptr_.0 }, sbuf, cond, callback, &strand, cur);
                            },
                            Err(err) => {
                                strand.io_service().post_strand(
                                    move |strand| callback(strand, Err(err)), &strand);
                            },
                        }
                    }, strand);
                },
                Err(err) => {
                    soc.io_service().post_strand(
                        move |strand| callback(strand, Err(err)), strand);
                },
            }
        }
    }
}


pub fn async_read_until<P: Protocol, B, C, F, T>(soc: &StreamSocket<P>, sbuf: B, cond: C, callback: F, strand: &Strand<T>)
    where B: FnOnce(&T) -> &mut StreamBuf + Send + 'static,
          C: MatchCondition + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    let sbuf = sbuf(strand);
    async_read_until_impl(soc, sbuf, cond, callback, strand, 0)
}

fn async_write_until_impl<S, F, T>(soc: &S, sbuf: &mut StreamBuf, mut cur: usize, callback: F, strand: &Strand<T>, mut sum: usize)
    where S: AsIoActor + NonBlocking + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    let ptr = sbuf as *mut StreamBuf;
    let buf = &sbuf.as_slice()[..cur];
    let mut ptr_ = UnsafeThreadableCell::new((soc as *const S, ptr));
    async_write(soc, buf, move |strand, res| {
        match res {
            Ok(len) => {
                let soc = unsafe { &*ptr_.0};
                let sbuf = unsafe { &mut *ptr_.1 };
                sbuf.consume(len);
                cur -= len;
                sum += len;
                if cur == 0 {
                    callback(strand, Ok(sum))
                } else {
                    async_write_until_impl(soc, sbuf, cur, callback, &strand, sum)
                }
            },
            Err(err) => {
                strand.io_service().post_strand(
                    move |strand| callback(strand, Err(err)), &strand)
            },
        }
    }, strand);
}

pub fn async_write_until<P: Protocol, B, C, F, T>(soc: &StreamSocket<P>, sbuf: B, mut cond: C, callback: F, strand: &Strand<T>)
    where B: FnOnce(&T) -> &mut StreamBuf + Send + 'static,
          C: MatchCondition + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    let sbuf = sbuf(strand);
    let len = match cond.is_match(sbuf.as_slice()) {
        Ok(len) => len,
        Err(len) => len,
    };
    async_write_until_impl(soc, sbuf, len, callback, strand, 0)
}
