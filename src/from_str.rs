use std::io;
use std::result;
use std::str::{Chars, FromStr};
use ip::{LlAddr,IpAddrV4,IpAddrV6};
use backbone::net_device::Ifreq;

#[derive(Debug)]
struct ParseError;

type Result<T> = result::Result<T, ParseError>;

trait Parser : Clone + Copy {
    type Output;
    fn parse<'a>(&self, it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)>;
}

fn address_family_not_supported() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "EAFNOSUPPORT")
}

#[derive(Clone, Copy)]
struct Lit(char);
impl Parser for Lit {
    type Output = ();

    fn parse<'a>(&self, mut it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        match it.next() {
            Some(ch) if ch == self.0 => Ok(((), it)),
            _ => Err(ParseError),
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
            _ => Err(ParseError),
        }
    }
}

#[derive(Clone, Copy)]
struct Char(&'static str);
impl Parser for Char {
    type Output = char;

    fn parse<'a>(&self, mut it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        if let Some(ch) = it.next() {
            let mut a = '\0';
            let mut chars = self.0.chars();
            while let Some(b) = chars.next() {
                if b == '-' {
                    if let Some(b) = chars.next() {
                        if a < ch && ch <= b {
                            return Ok((ch, it))
                        }
                    } else if ch == '-' {
                        return Ok((ch, it));
                    }
                } else if ch == b {
                    return Ok((ch, it))
                }
                a = b;
            }
        }
        Err(ParseError)
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
                _ => return Err(ParseError),
            },
            _ => return Err(ParseError),
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
            Err(ParseError)
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
                _ => return Err(ParseError),
            },
            _ => return Err(ParseError),
        };
        n = match it.next() {
            Some(ch) => match ch.to_digit(16) {
                Some(i) => n * 16 + i,
                _ => return Err(ParseError),
            },
            _ => return Err(ParseError),
        };
        if n <= 255 {
            Ok((n as u8, it))
        } else {
            Err(ParseError)
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
                _ => return Err(ParseError),
            },
            _ => return Err(ParseError),
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
            Err(ParseError)
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
        Err(ParseError)
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
struct SepBy<P, By>(P, By, usize);
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
            return Err(ParseError);
        }
        Ok((a, it))
    }
}

fn hex16_to_dec8(mut hex: u16) -> Option<u8> {
    let d = hex % 16;
    if d >= 10 { return None; }
    hex /= 16;
    let c = hex % 16;
    if c >= 10 { return None; }
    hex /= 16;
    let b = hex % 16;
    if b >= 10 { return None; }
    hex /= 16;
    let a = hex % 16;
    if a >= 10 { return None; }
    Some((((a * 10 + b) * 10 + c) * 10 + d) as u8)
}

#[derive(Clone, Copy)]
struct IpV6;
impl Parser for IpV6 {
    type Output = [u16; 8];

    fn parse<'a>(&self, mut it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        fn parse_ipv4<'a>(mut ar: [u16; 8], it: Chars<'a>) -> Result<([u16; 8], Chars<'a>)> {
            if ar[0] == 0 && ar[1] == 0 && ar[2] == 0 && ar[3] == 0 && ar[4] == 0
                && (ar[5] == 0 && ar[6] == 0 || ar[5] == 65535 && ar[6] == 0 || ar[5] == 0 && ar[6] == 65535)
            {
                if let Some(a) = hex16_to_dec8(ar[7]) {
                    if let Ok(((_, b), ne)) = Cat(Lit('.'), Dec8).parse(it.clone()) {
                        if let Ok(((_, c), ne)) = Cat(Lit('.'), Dec8).parse(ne) {
                            if let Ok(((_, d), ne)) = Cat(Lit('.'), Dec8).parse(ne) {
                                if ar[6] == 0xFFFF {
                                    ar[5] = 0xFFFF;
                                }
                                ar[6] = (a as u16 * 256) | b as u16;
                                ar[7] = (c as u16 * 256) | d as u16;
                                return Ok((ar, ne));
                            }
                        }
                    }
                }
            }
            Ok((ar, it))
        }

        fn parse_rev<'a>(mut ar: [u16; 8], i: usize, it: Chars<'a>) -> Result<([u16; 8], Chars<'a>)> {
            if let Ok((seps, it)) = SepBy(Hex16, Lit(':'), 7-i).parse(it.clone()) {
                for (i, hex) in seps.iter().rev().enumerate() {
                    ar[7-i] = *hex;
                }
                return parse_ipv4(ar, it);
            }

            Err(ParseError)
        }

        let mut ar = [0; 8];
        for i in 0..7 {
            if let Ok((_, it)) = Cat(Lit(':'), Lit(':')).parse(it.clone()) {
                return parse_rev(ar, i, it);
            } else if let Ok(((hex, _), ne)) = Cat(Hex16, Lit(':')).parse(it.clone()) {
                ar[i] = hex;
                it = ne;
                if let Ok((_, it)) = Lit(':').parse(it.clone()) {
                    return parse_rev(ar, i, it);
                }
            } else {
                return parse_rev(ar, i, it);
            }
        }

        if let Ok((hex, it)) = Hex16.parse(it.clone()) {
            ar[7] = hex;
            Ok((ar, it))
        } else if let Ok((_, it)) = Lit(':').parse(it) {
            Ok((ar, it))
        } else {
            Err(ParseError)
        }
    }
}

#[derive(Clone, Copy)]
struct ScopeId;
impl Parser for ScopeId {
    type Output = u32;

    fn parse<'a>(&self, it: Chars<'a>) -> Result<(Self::Output, Chars<'a>)> {
        if let Ok((_, it)) = Lit('%').parse(it.clone()) {
            if let Ok((ch, mut it)) = Char("a-zA-Z").parse(it.clone()) {
                let mut vec: Vec<u8> = Vec::new();
                vec.push(ch as u8);
                while let Ok((ch, ne)) = Char("0-9a-zA-Z.:_-").parse(it.clone()) {
                    vec.push(ch as u8);
                    it = ne;
                }
                if let Ok(ifr) = Ifreq::new(vec) {
                    if let Ok(id) = ifr.get_index() {
                        return Ok((id, it));
                    }
                }
            }
            if let Ok((dec, it)) = Dec8.parse(it.clone()) {
                return Ok((dec as u32, it));
            }
        }
        Ok((0, it))
    }
}

impl FromStr for LlAddr {
    type Err = io::Error;

    fn from_str(s: &str) -> io::Result<LlAddr> {
        if let Ok((addr, _)) = Eos(Sep6By(Hex08, LitOr('-', ':'))).parse(s.chars()) {
            Ok(LlAddr::new(addr[0], addr[1], addr[2], addr[3], addr[4], addr[5]))
        } else {
            Err(address_family_not_supported())
        }
    }
}

impl FromStr for IpAddrV4 {
    type Err = io::Error;

    fn from_str(s: &str) -> io::Result<IpAddrV4> {
        if let Ok((addr, _)) = Eos(Sep4By(Dec8, Lit('.'))).parse(s.chars()) {
            Ok(IpAddrV4::new(addr[0], addr[1], addr[2], addr[3]))
        } else {
            Err(address_family_not_supported())
        }
    }
}

impl FromStr for IpAddrV6 {
    type Err = io::Error;

    fn from_str(s: &str) -> io::Result<IpAddrV6> {
        if let Ok(((addr, id), _)) = Eos(Cat(IpV6, ScopeId)).parse(s.chars()) {
            Ok(IpAddrV6::with_scope_id(addr[0], addr[1], addr[2], addr[3],addr[4], addr[5], addr[6], addr[7], id))
        } else {
            Err(address_family_not_supported())
        }
    }
}

#[test]
fn test_lit() {
    assert_eq!(Lit('.').parse(".0".chars()).unwrap().0, ());
    assert_eq!(Lit(':').parse(":1".chars()).unwrap().0, ());
}

#[test]
fn test_lit_or() {
    assert_eq!(LitOr(':', '-').parse("-1".chars()).unwrap().0, ());
    assert_eq!(LitOr(':', '-').parse(":2".chars()).unwrap().0, ());
}

#[test]
fn test_char() {
    assert_eq!(Char("abc").parse("a".chars()).unwrap().0, 'a');
    assert_eq!(Char("abc").parse("b".chars()).unwrap().0, 'b');
    assert_eq!(Char("a-z").parse("z".chars()).unwrap().0, 'z');
    assert_eq!(Char("a-f0-9").parse("4".chars()).unwrap().0, '4');
    assert_eq!(Char("0-0A-Za-z-").parse("-".chars()).unwrap().0, '-');
    assert!(Char("abc").parse("d".chars()).is_err());
}

#[test]
fn test_dec8() {
    let p = Dec8;
    assert_eq!(p.parse("0".chars()).unwrap().0, 0);
    assert_eq!(p.parse("010".chars()).unwrap().0, 10);
    assert_eq!(p.parse("123".chars()).unwrap().0, 123);
    assert_eq!(p.parse("255".chars()).unwrap().0, 255);
    assert!(p.parse("256".chars()).is_err());
}

#[test]
fn test_hex08() {
    let p = Hex08;
    assert_eq!(p.parse("00".chars()).unwrap().0, 0);
    assert_eq!(p.parse("10".chars()).unwrap().0, 16);
    assert_eq!(p.parse("fF".chars()).unwrap().0, 255);
    assert!(p.parse("0".chars()).is_err());
    assert!(p.parse("f".chars()).is_err());
    assert!(p.parse("-1".chars()).is_err());
    assert!(p.parse("GF".chars()).is_err());
}

#[test]
fn test_hex16() {
    let p = Hex16;
    assert_eq!(p.parse("0".chars()).unwrap().0, 0);
    assert_eq!(p.parse("f".chars()).unwrap().0, 15);
    assert_eq!(p.parse("10".chars()).unwrap().0, 16);
    assert_eq!(p.parse("fF".chars()).unwrap().0, 255);
    assert_eq!(p.parse("FfFf".chars()).unwrap().0, 65535);
    assert!(p.parse("-1".chars()).is_err());
    assert!(p.parse("GF".chars()).is_err());
}

#[test]
fn test_cat() {
    let p = Cat(Hex16, Lit(':'));
    assert_eq!((p.parse("0:".chars()).unwrap().0).0, 0);
    assert!((p.parse("0-".chars()).is_err()));
    assert_eq!((p.parse("10:".chars()).unwrap().0).0, 16);
    assert_eq!((p.parse("ff:".chars()).unwrap().0).0, 255);
    assert_eq!((p.parse("ffff:".chars()).unwrap().0).0, 65535);
    assert!((p.parse("fffff:".chars()).is_err()));
}

#[test]
fn test_lladdr() {
    assert_eq!(LlAddr::from_str("00:00:00:00:00:00").unwrap(), LlAddr::new(0,0,0,0,0,0));
    assert_eq!(LlAddr::from_str("FF:ff:FF:fF:Ff:ff").unwrap(), LlAddr::new(255,255,255,255,255,255));
}

#[test]
fn test_ipv6() {
    let p = IpV6;
    assert_eq!(p.parse("1:2:3:4:5:6:7:8".chars()).unwrap().0, [1,2,3,4,5,6,7,8]);
    assert_eq!(p.parse("::".chars()).unwrap().0, [0;8]);
    assert_eq!(p.parse("::1".chars()).unwrap().0, [0,0,0,0,0,0,0,1]);
    assert_eq!(p.parse("::ffff:1".chars()).unwrap().0, [0,0,0,0,0,0,0xFFFF,1]);
    assert_eq!(p.parse("::2:3:4:5:6:7:8".chars()).unwrap().0, [0,2,3,4,5,6,7,8]);
    assert_eq!(p.parse("1::".chars()).unwrap().0, [1,0,0,0,0,0,0,0]);
    assert_eq!(p.parse("1::8".chars()).unwrap().0, [1,0,0,0,0,0,0,8]);
    assert_eq!(p.parse("1:2::8".chars()).unwrap().0, [1,2,0,0,0,0,0,8]);
    assert_eq!(p.parse("1::7:8".chars()).unwrap().0, [1,0,0,0,0,0,7,8]);
    assert_eq!(p.parse("1:2:3:4:5:6:7::".chars()).unwrap().0, [1,2,3,4,5,6,7,0]);
    assert_eq!(p.parse("0:0:0:0:0:0:255.255.255.255".chars()).unwrap().0, [0,0,0,0,0,0,0xFFFF,0xFFFF]);
    assert_eq!(p.parse("::0.255.255.0".chars()).unwrap().0, [0,0,0,0,0,0,0xFF,0xFF00]);
    assert_eq!(p.parse("0:0:0:0:0:FFFF:255.255.255.255".chars()).unwrap().0, [0,0,0,0,0,0xFFFF,0xFFFF,0xFFFF]);
    assert_eq!(p.parse("::FFFF:0.255.255.0".chars()).unwrap().0, [0,0,0,0,0,0xFFFF,0xFF,0xFF00]);
    assert_eq!(p.parse("1:2:3:4:5:6:7:8:9".chars()).unwrap().0, [1,2,3,4,5,6,7,8]);
    assert_eq!(p.parse("::2:3:4:5:6:7:8:9".chars()).unwrap().0, [0,2,3,4,5,6,7,8]);
    assert_eq!(p.parse("::2:3:4:5:6:7:8:9".chars()).unwrap().0, [0,2,3,4,5,6,7,8]);
    assert_eq!(p.parse("0:0:0:0:0:0:255.0.0.255.255".chars()).unwrap().0, [0,0,0,0,0,0,0xFF00,0xFF]);
    assert_eq!(p.parse("::255.0.0.255.255".chars()).unwrap().0, [0,0,0,0,0,0,0xFF00,0xFF]);
    assert_eq!(p.parse("0:0:0:0:0:ffff:255.0.0.255.255".chars()).unwrap().0, [0,0,0,0,0,0xFFFF,0xFF00,0xFF]);
    assert_eq!(p.parse("::ffff:255.0.0.255.255".chars()).unwrap().0, [0,0,0,0,0,0xFFFF,0xFF00,0xFF]);
}

#[test]
fn test_ipaddr_v4() {
    assert_eq!(IpAddrV4::from_str("0.0.0.0").unwrap(), IpAddrV4::new(0,0,0,0));
    assert_eq!(IpAddrV4::from_str("1.2.3.4").unwrap(), IpAddrV4::new(1,2,3,4));
}

#[test]
fn test_ipaddr_v6() {
    assert_eq!(IpAddrV6::from_str("1:2:3:4:5:6:7:8").unwrap(), IpAddrV6::new(1,2,3,4,5,6,7,8));
    assert!(IpAddrV6::from_str("1:2:3:4:5:6:7:8:9").is_err());
    assert_eq!(IpAddrV6::from_str("::").unwrap(), IpAddrV6::any());
    assert_eq!(IpAddrV6::from_str("::192.168.0.1").unwrap(), IpAddrV6::v4_compatible(&IpAddrV4::new(192,168,0,1)).unwrap());
    assert!(IpAddrV6::from_str("::192.168.0.1.1").is_err());
    assert_eq!(IpAddrV6::from_str("::ffff:0.0.0.0").unwrap(), IpAddrV6::v4_mapped(&IpAddrV4::any()));
    assert!(IpAddrV6::from_str("::1:192.168.0.1").is_err());
    assert_eq!(IpAddrV6::from_str("1:2:3:4:5:6:7:8%10").unwrap(), IpAddrV6::with_scope_id(1,2,3,4,5,6,7,8, 10));
    assert!(IpAddrV6::from_str("1:2:3:4:5:6:7:8%lo").unwrap().get_scope_id() != 0);
}
