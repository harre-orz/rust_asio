
fn match_cond_bytes_unchecked(buf: &[u8], head: u8, tail: &[u8]) -> Result<usize, usize> {
    let mut cur = 0;
    let mut it = buf.iter();
    while let Some(mut len) = it.position(|&ch| ch == head) {
        len += cur + 1;
        let buf = &buf[len..];
        if buf.len() < tail.len() {
            return Err(len - 1)
        } else if buf.starts_with(tail) {
            return Ok(len + tail.len())
        }
        cur = len;
        it = buf.iter();
    }
    Err(buf.len())
}


pub trait MatchCond : Send + 'static {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize>;
}

impl MatchCond for &'static [u8] {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        if self.is_empty() {
            Err(0)
        } else {
            match_cond_bytes_unchecked(buf, self[0], &self[1..])
        }
    }
}

impl MatchCond for &'static str {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        self.as_bytes().match_cond(buf)
    }
}

impl MatchCond for char {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        use std::mem;
        let mut bytes: [u8; 4] = unsafe { mem::uninitialized() };
        let len = self.encode_utf8(&mut bytes).as_bytes().len();
        match_cond_bytes_unchecked(buf, bytes[0], &bytes[1..len])
    }
}

impl MatchCond for String {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        match_cond_bytes_unchecked(buf, self.as_bytes()[0], &self.as_bytes()[1..])
    }
}

impl MatchCond for usize {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        if buf.len() > *self {
            Ok(*self)
        } else {
            *self -= buf.len();
            Err(buf.len())
        }
    }
}


#[test]
fn test_match_cond() {
    assert!((5 as usize).match_cond("hello".as_bytes()) == Ok(5));
    assert!((5 as usize).match_cond("hello world".as_bytes()) == Ok(5));
    assert!((10 as usize).match_cond("hello".as_bytes()) == Err(5));
    assert!('l'.match_cond("hello".as_bytes()) == Ok(3));
    assert!('w'.match_cond("hello".as_bytes()) == Err(5));
    assert!("lo".match_cond("hello world".as_bytes()) == Ok(5));
    assert!("world!".match_cond("hello world".as_bytes()) == Err(6));
    assert!("".match_cond("hello".as_bytes()) == Err(0));
    assert!("l".match_cond("hello".as_bytes()) == Ok(3));
}
