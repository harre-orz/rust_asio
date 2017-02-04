pub trait MatchCondition : Send + 'static {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize>;
}

impl MatchCondition for usize {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        if buf.len() >= *self {
            Ok(*self)
        } else {
            *self -= buf.len();
            Err(buf.len())
        }
    }
}

impl MatchCondition for u8 {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        if let Some(len) = buf.iter().position(|&x| x == *self) {
            Ok(len+1)
        } else {
            Err(buf.len())
        }
    }
}

impl MatchCondition for &'static [u8] {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        let mut cur = 0;
        if !self.is_empty() {
            let head = self[0];
            let tail = &self[1..];
            let mut it = buf.iter();
            while let Some(mut len) = it.position(|&x| x == head) {
                len += cur + 1;
                let buf = &buf[len..];
                if buf.len() < tail.len() {
                    return Err(len - 1);
                } else if buf.starts_with(tail) {
                    return Ok(len + tail.len());
                }
                cur = len;
                it = buf.iter();
            }
            cur = buf.len();
        }
        Err(cur)
    }
}

impl MatchCondition for char {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        (*self as u8).match_cond(buf)
    }
}

impl MatchCondition for &'static str {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        self.as_bytes().match_cond(buf)
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
