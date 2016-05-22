use std::io;
use std::cmp;
use Strand;
use socket::{StreamBuf, ReadWrite, MatchCondition};

pub fn read_until<S: ReadWrite, C: MatchCondition>(soc: &S, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
    let mut cur = 0;
    loop {
        match cond.is_match(&sbuf.as_slice()[cur..]) {
            Ok(len) => return Ok(cur + len),
            Err(len) => {
                cur = cmp::min(cur+len, sbuf.len());
                let len = try!(soc.read_some(try!(sbuf.prepare(4096))));
                sbuf.commit(len);
            },
        }
    }
}

pub fn async_read_until<S: ReadWrite, C: MatchCondition, A, F, T>(a: A, mut cond: T, callback: F, obj: &Strand<T>)
    where A: Fn(&mut T) -> (&S, &mut StreamBuf),
          F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    unimplemented!();
}

pub fn write_until<S: ReadWrite, C: MatchCondition>(soc: &S, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
    let len = {
        let len = match cond.is_match(sbuf.as_slice()) {
            Ok(len) => len,
            Err(len) => len,
        };
        try!(soc.write_some(&sbuf.as_slice()[..cmp::min(len, sbuf.len())]))
    };
    sbuf.consume(len);
    Ok(len)
}

pub fn async_write_until<S: ReadWrite, C: MatchCondition, A, F, T>(a: A, mut cond: T, callback: F, obj: &Strand<T>)
    where A: Fn(&mut T) -> (&S, &mut StreamBuf),
          F: FnOnce(&Strand<T>, io::Result<usize>) + Send + 'static,
          T: 'static {
    unimplemented!();
}
