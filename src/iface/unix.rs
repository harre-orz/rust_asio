use crate::error::{OsError, Result};
use crate::sockaddr::SockAddrPhysical;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::{mem, ptr};

#[derive(Copy, Clone, Debug)]
pub struct Iface {
    ifi: libc::c_uint,
}

impl Iface {
    pub fn new(if_name: &str) -> Result<Iface> {
        if let Ok(if_name) = CString::new(if_name) {
            unsafe {
                match libc::if_nametoindex(if_name.as_ptr().cast()) {
                    0 => Err(OsError::last()),
                    ifi => Ok(Self { ifi: ifi }),
                }
            }
        } else {
            Err(OsError::NO_SUCH_DEVICE)
        }
    }

    pub const unsafe fn from_raw(ifi: libc::c_uint) -> Self {
        Self { ifi: ifi }
    }

    pub const fn as_raw(&self) -> libc::c_uint {
        self.ifi
    }
}

const fn ipv4_netmask_to_prefix(ipv4: &Ipv4Addr) -> u8 {
    ipv4.to_bits().leading_ones() as u8
}

const fn ipv6_netmask_to_prefix(ipv6: &Ipv6Addr) -> u8 {
    ipv6.to_bits().leading_ones() as u8
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum IfaceAddrRef<'a> {
    V4(&'a Ipv4Addr, u8),
    V6(&'a Ipv6Addr, u8),
    Hw(&'a SockAddrPhysical),
}

pub struct IfaceRef<'a>(&'a libc::ifaddrs);

impl<'a> IfaceRef<'a> {
    pub fn name(&self) -> &str {
        unsafe {
            let name = CStr::from_ptr(self.0.ifa_name);
            str::from_utf8_unchecked(name.to_bytes())
        }
    }

    pub fn addr(&self) -> IfaceAddrRef<'_> {
        unsafe {
            let sa = &*self.0.ifa_addr;
            match sa.sa_family as i32 {
                libc::AF_INET => {
                    let sin = &*(self.0.ifa_addr as *const libc::sockaddr_in);
                    let mask = &*(self.0.ifa_netmask as *const libc::sockaddr_in);
                    let mask: &Ipv4Addr = mem::transmute(&mask.sin_addr);
                    let len = ipv4_netmask_to_prefix(mask);
                    IfaceAddrRef::V4(mem::transmute(&sin.sin_addr), len)
                }
                libc::AF_INET6 => {
                    let sin6 = &*(self.0.ifa_addr as *const libc::sockaddr_in6);
                    let mask = &*(self.0.ifa_netmask as *const libc::sockaddr_in6);
                    let mask: &Ipv6Addr = mem::transmute(&mask.sin6_addr);
                    let len = ipv6_netmask_to_prefix(mask);
                    IfaceAddrRef::V6(mem::transmute(&sin6.sin6_addr), len)
                }
                #[cfg(target_os = "linux")]
                libc::AF_PACKET => {
                    let sll = &*(self.0.ifa_addr as *const libc::sockaddr_ll);
                    IfaceAddrRef::Hw(mem::transmute(sll))
                }
                #[cfg(target_os = "macos")]
                libc::AF_LINK => {
                    let sdl = &*(self.0.ifa_addr as *const libc::sockaddr_dl);
                    IfaceAddrRef::Hw(mem::transmute(sdl))
                }
                _ => unreachable!(),
            }
        }
    }

    pub fn is_up(&self) -> bool {
        self.0.ifa_flags & libc::IFF_UP as u32 != 0
    }

    pub fn is_loopback(&self) -> bool {
        self.0.ifa_flags & libc::IFF_LOOPBACK as u32 != 0
    }

    pub fn is_running(&self) -> bool {
        self.0.ifa_flags & libc::IFF_RUNNING as u32 != 0
    }
}

pub struct IfacesIter<'a>(*mut libc::ifaddrs, PhantomData<&'a ()>);

impl<'a> Iterator for IfacesIter<'a> {
    type Item = IfaceRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == ptr::null_mut() {
            None
        } else {
            let ifa = self.0;
            self.0 = unsafe { (*self.0).ifa_next };
            Some(IfaceRef(unsafe { &*ifa }))
        }
    }
}

pub struct Ifaces {
    ifa: *mut libc::ifaddrs,
}

impl Drop for Ifaces {
    fn drop(&mut self) {
        unsafe {
            libc::freeifaddrs(self.ifa);
        }
    }
}

impl Ifaces {
    pub fn new() -> Result<Ifaces> {
        let mut ifa = ptr::null_mut();
        unsafe {
            match libc::getifaddrs(&mut ifa) {
                -1 => Err(OsError::last()),
                _ => Ok(Ifaces { ifa: ifa }),
            }
        }
    }

    pub fn iter(&'_ self) -> IfacesIter<'_> {
        IfacesIter(self.ifa, PhantomData)
    }
}

#[test]
fn test_v4_prefix() {
    let len = ipv4_netmask_to_prefix(&Ipv4Addr::new(0, 0, 0, 0));
    assert_eq!(len, 0);

    let len = ipv4_netmask_to_prefix(&Ipv4Addr::new(255, 0, 0, 0));
    assert_eq!(len, 8);

    let len = ipv4_netmask_to_prefix(&Ipv4Addr::new(255, 255, 0, 0));
    assert_eq!(len, 16);

    let len = ipv4_netmask_to_prefix(&Ipv4Addr::new(255, 255, 255, 0));
    assert_eq!(len, 24);

    let len = ipv4_netmask_to_prefix(&Ipv4Addr::new(255, 255, 255, 252));
    assert_eq!(len, 30);

    let len = ipv4_netmask_to_prefix(&Ipv4Addr::new(255, 255, 255, 255));
    assert_eq!(len, 32);
}

#[test]
fn test_v6_prefix() {
    let len = ipv6_netmask_to_prefix(&Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0));
    assert_eq!(len, 0);

    let len = ipv6_netmask_to_prefix(&Ipv6Addr::new(0xffff, 0xffff, 0xffff, 0xffff, 0, 0, 0, 0));
    assert_eq!(len, 64);

    let len = ipv6_netmask_to_prefix(&Ipv6Addr::new(
        0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xf000,
    ));
    assert_eq!(len, 116);

    let len = ipv6_netmask_to_prefix(&Ipv6Addr::new(
        0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
    ));
    assert_eq!(len, 128);
}
