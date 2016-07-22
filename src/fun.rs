use std::io;
use {IoObject, Strand, Protocol, NonBlocking, ConstBuffer, MutableBuffer, Stream, StreamBuf, MatchCondition};
use {UnsafeThreadableCell};
use ops::async::*;

pub fn read_until<S: Stream, C: MatchCondition>(soc: &S, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
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

pub fn write_until<S: Stream, C: MatchCondition>(soc: &S, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
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

unsafe fn async_read_until_impl<S, C, F, T>(soc: &S, sbuf: &mut StreamBuf, mut cond: C, callback: F, strand: &Strand<T>, mut cur: usize)
    where S: Stream + Send + 'static,
          C: MatchCondition + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    match cond.is_match(&sbuf.as_slice()[cur..]) {
        Ok(len) => {
            strand.io_service().post_strand(
                move |strand| callback(strand, Ok(cur + len)), strand);
        },
        Err(len) => {
            cur += len;
            let ptr = sbuf as *mut StreamBuf;
            match sbuf.prepare(4096) {
                Ok(buf) => {
                    let mut ptr_ = UnsafeThreadableCell::new((soc as *const S, ptr));
                    soc.async_read_some(unsafe { MutableBuffer::new(buf) }, move |strand, res| {
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
                    strand.io_service().post_strand(
                        move |strand| callback(strand, Err(err)), strand);
                },
            }
        }
    }
}

pub unsafe fn async_read_until<S, C, F, T>(soc: &S, sbuf: &StreamBuf, cond: C, callback: F, strand: &Strand<T>)
    where S: Stream + Send + 'static,
          C: MatchCondition + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    let sbuf = sbuf as *const StreamBuf as *mut StreamBuf;
    async_read_until_impl(soc, unsafe { &mut *sbuf }, cond, callback, strand, 0)
}

unsafe fn async_write_until_impl<S, F, T>(soc: &S, sbuf: &mut StreamBuf, mut cur: usize, callback: F, strand: &Strand<T>, mut sum: usize)
    where S: Stream + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    let mut ptr = UnsafeThreadableCell::new((soc as *const S, sbuf as *mut StreamBuf));
    soc.async_write_some(unsafe { ConstBuffer::new(&sbuf.as_slice()[..cur]) }, move |strand, res| {
        match res {
            Ok(len) => {
                let soc = unsafe { &*ptr.0};
                let sbuf = unsafe { &mut *ptr.1 };
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

pub unsafe fn async_write_until<S, C, F, T>(soc: &S, sbuf: &StreamBuf, mut cond: C, callback: F, strand: &Strand<T>)
    where S: Stream + Send + 'static,
          C: MatchCondition + Send + 'static,
          F: FnOnce(Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    let len = match cond.is_match(sbuf.as_slice()) {
        Ok(len) => len,
        Err(len) => len,
    };
    let sbuf = sbuf as *const StreamBuf as *mut StreamBuf;
    async_write_until_impl(soc, unsafe { &mut *sbuf }, len, callback, strand, 0)
}
