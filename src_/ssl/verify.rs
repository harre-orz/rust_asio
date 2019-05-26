use ip::IpAddr;

use std::slice;
use std::str::FromStr;
use std::ascii::AsciiExt;
use openssl::nid::Nid;
use openssl::x509::*;

pub type SslVerifyContext = X509StoreContextRef;

fn match_pattern(mut patt: slice::Iter<u8>, mut host: slice::Iter<u8>) -> bool {
    while let Some(p) = patt.next() {
        let p = *p as char;
        let mut h = if let Some(h) = host.next() {
            *h as char
        } else {
            return false;
        };

        if p == '*' {
            while h != '.' {
                if match_pattern(patt.clone(), host.clone()) {
                    return true;
                }
                if let Some(h2) = host.next() {
                    h = *h2 as char;
                } else {
                    break;
                }
            }
        }
        else if !p.eq_ignore_ascii_case(&h) {
            return false;
        }
    }
    true
}

pub struct Rfc2818Verification(pub String);

impl Rfc2818Verification {
    pub fn verification(&self, preverified: bool, ctx: SslVerifyContext) -> bool {
        if !preverified {
            return false;
        }

        let depth = ctx.error_depth();
        if depth > 0 {
            return true;
        }

        let addr = IpAddr::from_str(&self.0);
        let cert = ctx.current_cert().unwrap();

        for gen in cert.subject_alt_names().unwrap() {
            if let &Ok(ref addr) = &addr {
                if let Some(bytes) = gen.ipaddress() {
                    if addr.as_bytes() == bytes {
                        return true;
                    }
                }
            } else {
                if let Some(domain) = gen.dnsname() {
                    if match_pattern(domain.as_bytes().iter(), self.0.as_bytes().iter()) {
                        return true;
                    }
                }
            }
        }

        let name = cert.subject_name();
        for e in name.entries_by_nid(Nid::from_raw(-1)) {
            let asn1str = e.data();
            if match_pattern(asn1str.as_slice().iter(), self.0.as_bytes().iter()) {
                return true;
            }
        }

        false
    }
}

#[test]
fn test_match_pattern() {
    assert_eq!(match_pattern("example.com".as_bytes().iter(), "example.com".as_bytes().iter()), true);
    assert_eq!(match_pattern("*.com".as_bytes().iter(), "example.com".as_bytes().iter()), true);
    assert_eq!(match_pattern("*".as_bytes().iter(), "example.com".as_bytes().iter()), true);
    assert_eq!(match_pattern("example.jp".as_bytes().iter(), "example.com".as_bytes().iter()), false);
    assert_eq!(match_pattern("example.jp".as_bytes().iter(), "example.com".as_bytes().iter()), false);
    assert_eq!(match_pattern("*.example.com".as_bytes().iter(), "example.com".as_bytes().iter()), false);
}
