use std::io;
use std::result;
use std::str::{Chars, FromStr};
use socket::ip::{LlAddr,IpAddrV4,IpAddrV6};

#[derive(Debug)]
struct AddrParseError;

type Result<T> = result::Result<T, AddrParseError>;

trait Parser : Clone + Copy {
    type Output;
    fn parse<'a>(&self, it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)>;
}

#[derive(Clone, Copy)]
struct Lit(char);
impl Parser for Lit {
    type Output = ();

    fn parse<'a>(&self, mut it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        match it.next() {
            Some(ch) if ch == self.0 => Ok(((), it)),
            _ => Err(AddrParseError),
        }
    }
}

#[derive(Clone, Copy)]
struct LitOr(char, char);
impl Parser for LitOr {
    type Output = ();

    fn parse<'a>(&self, mut it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)>  {
        match it.next() {
            Some(ch) if ch == self.0 || ch == self.1 => Ok(((), it)),
            _ => Err(AddrParseError),
        }
    }
}

#[derive(Clone, Copy)]
struct Dec8;
impl Parser for Dec8 {
    type Output = u8;

    fn parse<'a>(&self, mut it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        let mut n = match it.next() {
            Some(ch) => match ch.to_digit(10) {
                Some(i) => i,
                _ => return Err(AddrParseError),
            },
            _ => return Err(AddrParseError),
        };
        for _ in 0..2 {
            let p = it.clone();
            n = match it.next() {
                Some(ch) => match ch.to_digit(10) {
                    Some(i) => n * 10 + i,
                    _ => return Ok((n as u8, p))
                },
                _ => return Ok((n as u8, p)),
            };
        }
        if n <= 255 {
            Ok((n as u8, it))
        } else {
            Err(AddrParseError)
        }
    }
}

#[derive(Clone, Copy)]
struct Hex08;
impl Parser for Hex08 {
    type Output = u8;

    fn parse<'a>(&self, mut it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        let mut n = match it.next() {
            Some(ch) => match ch.to_digit(16) {
                Some(i) => i,
                _ => return Err(AddrParseError),
            },
            _ => return Err(AddrParseError),
        };
        n = match it.next() {
            Some(ch) => match ch.to_digit(16) {
                Some(i) => n * 16 + i,
                _ => return Err(AddrParseError),
            },
            _ => return Err(AddrParseError),
        };
        if n <= 255 {
            Ok((n as u8, it))
        } else {
            Err(AddrParseError)
        }
    }
}

#[derive(Clone, Copy)]
struct Hex16;
impl Parser for Hex16 {
    type Output = u16;

    fn parse<'a>(&self, mut it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        let mut n = match it.next() {
            Some(ch) => match ch.to_digit(16) {
                Some(i) => i,
                _ => return Err(AddrParseError),
            },
            _ => return Err(AddrParseError),
        };
        for _ in 0..4 {
            let p = it.clone();
            n = match it.next() {
                Some(ch) => match ch.to_digit(16) {
                    Some(i) => n * 16 + i,
                    _ => return Ok((n as u16, p)),
                },
                _ => return Ok((n as u16, p)),
            };
        }
        if n < 65536 {
            Ok((n as u16, it))
        } else {
            Err(AddrParseError)
        }
    }
}

#[derive(Clone, Copy)]
struct IpV6;
impl Parser for IpV6 {
    type Output = [u16; 8];

    fn parse<'a>(&self, it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        //                    a[i]            | b[j]
        // 1:2:3:4:5:6:7:8 -> 1:2:3:4:5:6:7:8 | Ok
        // 1:2::4:5        -> 1:2::           | 4:5
        // 1::1            -> 1::             | 1
        // ::              -> ::              | Nil
        // :2:3:4:5:6:7:8  ->                 | 2:3:4:5:6:7:8

        fn parse_a<'a>(mut it: Chars<'a>) -> Result<([u16; 8], usize, Chars<'a>)> {
            let mut a = [0; 8];
            let mut i = 0;
            while i < 7 {
                if let Ok(((hex, _), ne)) = Cat(Hex16, Lit(':')).parse(it.clone()) {
                    a[i] = hex;
                    it = ne;
                } else if let Ok((_, ne)) = Lit(':').parse(it.clone()) {
                    println!("parse_a rest{}", it.as_str());
                    return Ok((a, i, ne));
                } else {
                    println!("parse_a rest{}", it.as_str());
                    return Ok((a, i, it));
                }
                i += 1;
            }
            if let Ok((hex, ne)) = Hex16.parse(it) {
                a[i] = hex;
                println!("parse_a comp");
                return Ok((a, 8, ne));
            } else {
                return Err(AddrParseError);
            }
        }

        fn parse_b<'a>(mut it: Chars<'a>) -> Result<([u16; 8], Chars<'a>)> {
            println!("start {}", it.as_str());
            if let Ok((mut a, i, it)) = parse_a(it) {
                println!("rest {}", it.as_str());

                if i == 8 {
                    return Ok((a, it));
                } else if let Ok((b, it)) = SepBy(Hex16, Lit(':'), 7-i).parse(it.clone()) {
                    for (ch, i) in b.iter().rev().zip(0..7) {
                        a[7-i] = *ch;
                    }
                }
                Ok((a, it))
            } else {
                Err(AddrParseError)
            }
        }

        if let Ok((arr, it)) = parse_b(it) {
            println!("{}", IpAddrV6::new(arr[0], arr[1], arr[2], arr[3],arr[4], arr[5], arr[6], arr[7], 0));
            Ok((arr, it))
        } else {
            Err(AddrParseError)
        }
    }
}

#[derive(Clone, Copy)]
struct Cat<P1, P2>(P1, P2);
impl<P1: Parser, P2: Parser> Parser for Cat<P1, P2> {
    type Output = (P1::Output, P2::Output);

    fn parse<'a>(&self, it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        if let Ok((a, it)) = self.0.parse(it) {
            if let Ok((b, it)) = self.1.parse(it) {
                return Ok(((a, b), it));
            }
        }
        Err(AddrParseError)
    }
}

#[derive(Clone, Copy)]
struct Sep4By<P: Parser, By: Parser>(P, By);
impl<P: Parser, By: Parser> Parser for Sep4By<P, By> {
    type Output = [P::Output; 4];

    fn parse<'a>(&self, it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        let (a, it) = try!(self.0.parse(it));
        let (_, it) = try!(self.1.parse(it));
        let (b, it) = try!(self.0.parse(it));
        let (_, it) = try!(self.1.parse(it));
        let (c, it) = try!(self.0.parse(it));
        let (_, it) = try!(self.1.parse(it));
        let (d, it) = try!(self.0.parse(it));
        Ok(([a,b,c,d], it))
    }
}

#[derive(Clone, Copy)]
struct Sep6By<P, By>(P, By);
impl<P: Parser, By: Parser> Parser for Sep6By<P, By> {
    type Output = [P::Output; 6];

    fn parse<'a>(&self, it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        let (a, it) = try!(self.0.parse(it));
        let (_, it) = try!(self.1.parse(it));
        let (b, it) = try!(self.0.parse(it));
        let (_, it) = try!(self.1.parse(it));
        let (c, it) = try!(self.0.parse(it));
        let (_, it) = try!(self.1.parse(it));
        let (d, it) = try!(self.0.parse(it));
        let (_, it) = try!(self.1.parse(it));
        let (e, it) = try!(self.0.parse(it));
        let (_, it) = try!(self.1.parse(it));
        let (f, it) = try!(self.0.parse(it));
        Ok(([a,b,c,d,e,f], it))
    }
}

#[derive(Clone, Copy)]
struct SepBy<P, By>(P, By,usize);
impl<P: Parser, By: Parser> Parser for SepBy<P, By> {
    type Output = Vec<P::Output>;

    fn parse<'a>(&self, mut it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        let mut vec = Vec::new();

        if let Ok((a, ne)) = self.0.parse(it.clone()) {
            it = ne;
            vec.push(a);

            while vec.len() < self.2 {
                if let Ok(((_, a), ne)) = Cat(self.1, self.0).parse(it.clone()) {
                    it = ne;
                    vec.push(a);
                } else {
                    break;
                }
            }
        }
        Ok((vec, it))
    }
}

#[derive(Clone, Copy)]
struct Between<A, P, B>(A, P, B);
impl<A: Parser, P: Parser, B: Parser> Parser for Between<A, P, B> {
    type Output = P::Output;

    fn parse<'a>(&self, it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        let (_, it) = try!(self.0.parse(it));
        let (a, it) = try!(self.1.parse(it));
        let (_, it) = try!(self.2.parse(it));
        Ok((a, it))
    }
}

#[derive(Clone, Copy)]
struct Eos<P>(P);
impl<P: Parser> Parser for Eos<P> {
    type Output = P::Output;

    fn parse<'a>(&self, it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        let (a, mut it) = try!(self.0.parse(it));
        if let Some(_) = it.next() {
            return Err(AddrParseError);
        }
        Ok((a, it))
    }
}

impl FromStr for LlAddr {
    type Err = io::Error;

    fn from_str(s: &str) -> io::Result<LlAddr> {
        if let Ok((addr, _)) = Eos(Sep6By(Hex08, LitOr('-', ':'))).parse(s.chars()) {
            Ok(LlAddr::new(addr[0], addr[1], addr[2], addr[3], addr[4], addr[5]))
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "EAFNOSUPPORT"))
        }
    }
}

impl FromStr for IpAddrV4 {
    type Err = io::Error;

    fn from_str(s: &str) -> io::Result<IpAddrV4> {
        if let Ok((addr, _)) = Eos(Sep4By(Dec8, Lit('.'))).parse(s.chars()) {
            Ok(IpAddrV4::new(addr[0], addr[1], addr[2], addr[3]))
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "EAFNOSUPPORT"))
        }
    }
}

impl FromStr for IpAddrV6 {
    type Err = io::Error;

    fn from_str(s: &str) -> io::Result<IpAddrV6> {
        if let Ok((addr, _)) = Eos(IpV6).parse(s.chars()) {
            Ok(IpAddrV6::new(addr[0], addr[1], addr[2], addr[3],addr[4], addr[5], addr[6], addr[7], 0))
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "EAFNOSUPPORT"))
        }
    }
}

#[test]
fn test_lit() {
    assert!(Lit('.').parse(".0".chars()).unwrap().0 == ());
    assert!(Lit(':').parse(":1".chars()).unwrap().0 == ());
}

#[test]
fn test_lit_or() {
    assert!(LitOr(':', '-').parse("-1".chars()).unwrap().0 == ());
    assert!(LitOr(':', '-').parse(":2".chars()).unwrap().0 == ());
}

#[test]
fn test_dec8() {
    let p = Dec8;
    assert!(p.parse("0".chars()).unwrap().0 == 0);
    assert!(p.parse("010".chars()).unwrap().0 == 10);
    assert!(p.parse("123".chars()).unwrap().0 == 123);
    assert!(p.parse("255".chars()).unwrap().0 == 255);
    assert!(p.parse("256".chars()).is_err());
}

#[test]
fn test_hex08() {
    let p = Hex08;
    assert!(p.parse("00".chars()).unwrap().0 == 0);
    assert!(p.parse("10".chars()).unwrap().0 == 16);
    assert!(p.parse("fF".chars()).unwrap().0 == 255);
    assert!(p.parse("0".chars()).is_err());
    assert!(p.parse("f".chars()).is_err());
    assert!(p.parse("-1".chars()).is_err());
    assert!(p.parse("GF".chars()).is_err());
}

#[test]
fn test_hex16() {
    let p = Hex16;
    assert!(p.parse("0".chars()).unwrap().0 == 0);
    assert!(p.parse("f".chars()).unwrap().0 == 15);
    assert!(p.parse("10".chars()).unwrap().0 == 16);
    assert!(p.parse("fF".chars()).unwrap().0 == 255);
    assert!(p.parse("FfFf".chars()).unwrap().0 == 65535);
    assert!(p.parse("-1".chars()).is_err());
    assert!(p.parse("GF".chars()).is_err());
}

#[test]
fn test_cat() {
    let p = Cat(Hex16, Lit(':'));
    assert!((p.parse("0:".chars()).unwrap().0).0 == 0);
    assert!((p.parse("0-".chars()).is_err()));
    assert!((p.parse("10:".chars()).unwrap().0).0 == 16);
    assert!((p.parse("ff:".chars()).unwrap().0).0 == 255);
    assert!((p.parse("ffff:".chars()).unwrap().0).0 == 65535);
    assert!((p.parse("fffff:".chars()).is_err()));
}

#[test]
fn test_lladdr() {
    assert!(LlAddr::from_str("00:00:00:00:00:00").unwrap() == LlAddr::new(0,0,0,0,0,0));
}

#[test]
fn test_ipv6() {
    // let p = IpV6;
    // p.parse("1:2:3:4:5:6:7:8".chars());
    // println!("");
    // p.parse("1:2:3:4:5:6:7:".chars());
    // println!("");
    // p.parse("1:2::7:8".chars());
    // println!("");
    // p.parse("1:2::".chars());
    // println!("");
    // p.parse("::8".chars());
    // println!("");
    // p.parse("::".chars());
    // println!("");
    // assert!(false);
}

#[test]
fn test_ipaddr_v4() {
    assert!(IpAddrV4::from_str("0.0.0.0").unwrap() == IpAddrV4::new(0,0,0,0));
    assert!(IpAddrV4::from_str("1.2.3.4").unwrap() == IpAddrV4::new(1,2,3,4));
}
