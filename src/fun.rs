use super::*;
use std::io;
use std::cmp;

pub fn read_until<'a, S: ReadWrite<'a>, T: MatchCondition>(soc: &S, sbuf: &mut StreamBuf, mut cond: T) -> io::Result<usize> {
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

pub fn write_until<'a, S: ReadWrite<'a>, T: MatchCondition>(soc: &S, sbuf: &mut StreamBuf, mut cond: T) -> io::Result<usize> {
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
