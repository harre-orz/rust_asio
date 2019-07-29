//

use super::{IpAddrV4, IpAddrV6, LlAddr};

pub struct IfaceAddrs {
    pub name: String,
    pub ll_addr: LlAddr,
    pub ip_addr_v4: Vec<IpAddrV4>,
    pub ip_addr_v6: Vec<IpAddrV6>,
}

#[cfg(unix)]
mod specified {
    use super::{IfaceAddrs, IpAddrV4, IpAddrV6, LlAddr};
    use error::ErrorCode;
    use libc;
    use std::ffi::CStr;
    use std::io;
    use std::ptr;

    pub struct Iface {
        ifaces: Vec<IfaceAddrs>,
    }

    impl Iface {
        pub fn new() -> io::Result<Self> {
            unsafe {
                let mut ifaces: Vec<IfaceAddrs> = Vec::new();
                let mut ifaddr = ptr::null_mut();
                let err = libc::getifaddrs(&mut ifaddr);
                if err == -1 {
                    return Err(ErrorCode::last_error().into());
                }
                let mut ifa = ifaddr;
                while !ifa.is_null() {
                    let name = CStr::from_ptr((*ifa).ifa_name).to_str().unwrap();
                    let iface: &mut IfaceAddrs = {
                        if let Some(iface) = ifaces.iter_mut().find(|x| x.name == name) {
                            iface
                        } else {
                            ifaces.push(IfaceAddrs {
                                name: name.to_string(),
                                ll_addr: LlAddr::new(0, 0, 0, 0, 0, 0),
                                ip_addr_v4: Vec::new(),
                                ip_addr_v6: Vec::new(),
                            });
                            ifaces.iter_mut().last().unwrap()
                        }
                    };
                    match (*(*ifa).ifa_addr).sa_family as i32 {
                        libc::AF_PACKET => {
                            let sll = (*ifa).ifa_addr as *const libc::sockaddr_ll;
                            let sll: &[u8] = &(*sll).sll_addr;
                            iface.ll_addr =
                                LlAddr::new(sll[0], sll[1], sll[2], sll[3], sll[4], sll[5])
                        }
                        libc::AF_INET => {
                            let sin = (*ifa).ifa_addr as *const libc::sockaddr_in;
                            iface
                                .ip_addr_v4
                                .push(IpAddrV4::from((*sin).sin_addr.clone()))
                        }
                        libc::AF_INET6 => {
                            let sin6 = (*ifa).ifa_addr as *const libc::sockaddr_in6;
                            iface.ip_addr_v6.push(IpAddrV6 {
                                bytes: ((*sin6).sin6_addr.clone()).s6_addr,
                                scope_id: (*sin6).sin6_scope_id,
                            })
                        }
                        _ => unreachable!(),
                    };
                    ifa = (*ifa).ifa_next;
                }
                libc::freeifaddrs(ifaddr);
                Ok(Iface { ifaces: ifaces })
            }
        }

        pub fn get<T>(&self, name: T) -> Option<&IfaceAddrs>
        where
            T: AsRef<str>,
        {
            self.ifaces.iter().find(|x| x.name == name.as_ref())
        }
    }
}

pub use self::specified::*;

#[test]
fn test_iface_lo() {
    let ifaces = Iface::new().unwrap();
    let iface = ifaces.get("lo").unwrap();
    assert_eq!(iface.ip_addr_v4[0], IpAddrV4::loopback());
    assert_eq!(iface.ip_addr_v6[0], IpAddrV6::loopback());
}
