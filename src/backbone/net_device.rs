use std::io;
use std::mem;
use std::ptr;
use std::slice;
use std::ffi::CString;
use super::{errno, c_char, RawFd, socket, sockaddr, sockaddr_in};
use ip::{LlAddr, IpAddrV4, Udp};
use libc;

const IFNAMSIZ: usize = 16;

/* Socket configuration controls. */
// const SIOCGIFNAME: i32        = 0x8910;      /* get iface name       */
// const SIOCSIFLINK: i32        = 0x8911;      /* set iface channel        */
// const SIOCGIFCONF: i32        = 0x8912;      /* get iface list       */
const SIOCGIFFLAGS: i32       = 0x8913;      /* get flags            */
// const SIOCSIFFLAGS: i32       = 0x8914;      /* set flags            */
const SIOCGIFADDR: i32        = 0x8915;      /* get PA address       */
// const SIOCSIFADDR: i32        = 0x8916;      /* set PA address       */
// const SIOCGIFDSTADDR: i32     = 0x8917;      /* get remote PA address    */
// const SIOCSIFDSTADDR: i32     = 0x8918;      /* set remote PA address    */
// const SIOCGIFBRDADDR: i32     = 0x8919;      /* get broadcast PA address */
// const SIOCSIFBRDADDR: i32     = 0x891a;      /* set broadcast PA address */
// const SIOCGIFNETMASK: i32     = 0x891b;      /* get network PA mask      */
// const SIOCSIFNETMASK: i32     = 0x891c;      /* set network PA mask      */
// const SIOCGIFMETRIC: i32      = 0x891d;      /* get metric           */
// const SIOCSIFMETRIC: i32      = 0x891e;      /* set metric           */
// const SIOCGIFMEM: i32         = 0x891f;      /* get memory address (BSD) */
// const SIOCSIFMEM: i32         = 0x8920;      /* set memory address (BSD) */
const SIOCGIFMTU: i32         = 0x8921;      /* get MTU size         */
// const SIOCSIFMTU: i32         = 0x8922;      /* set MTU size         */
// const SIOCSIFNAME: i32        = 0x8923;      /* set interface name       */
// const SIOCSIFHWADDR: i32      = 0x8924;      /* set hardware address     */
// const SIOCGIFENCAP: i32       = 0x8925;      /* get/set encapsulations       */
// const SIOCSIFENCAP: i32       = 0x8926;
const SIOCGIFHWADDR: i32      = 0x8927;      /* Get hardware address     */
// const SIOCGIFSLAVE: i32       = 0x8929;      /* Driver slaving support   */
// const SIOCSIFSLAVE: i32       = 0x8930;
// const SIOCADDMULTI: i32       = 0x8931;      /* Multicast address lists  */
// const SIOCDELMULTI: i32       = 0x8932;
const SIOCGIFINDEX: i32       = 0x8933;      /* name -> if_index mapping */
// const SIOCSIFPFLAGS: i32      = 0x8934;      /* set/get extended flags set   */
// const SIOCGIFPFLAGS: i32      = 0x8935;
// const SIOCDIFADDR: i32        = 0x8936;      /* delete PA address        */
// const SIOCSIFHWBROADCAST: i32 = 0x8937;      /* set hardware broadcast addr  */
// const SIOCGIFCOUNT: i32       = 0x8938;      /* get number of devices */

#[repr(C)]
struct ifreq {
    ifr_name: [c_char; IFNAMSIZ],
    union: [u8; 24],
}

pub struct Ifreq {
    ifr: ifreq,
    fd: RawFd,
}

impl Ifreq {
    #[allow(dead_code)]
    pub fn new<T: Into<Vec<u8>>>(ifname: T) -> io::Result<Ifreq> {
        match CString::new(ifname) {
            Ok(ref s) if s.as_bytes().len() < IFNAMSIZ => {
                let fd = try!(socket(&Udp::v4()));
                let mut ifr = Ifreq {
                    ifr: unsafe { mem::uninitialized() },
                    fd: fd,
                };
                let src = s.as_bytes_with_nul();
                let dst = unsafe { slice::from_raw_parts_mut(ifr.ifr.ifr_name.as_mut_ptr() as *mut u8, src.len()) };
                dst.clone_from_slice(src);
                Ok(ifr)
            }
            _ =>
                Err(io::Error::new(io::ErrorKind::Other, "Unsupported interface-name"))
        }
    }

    #[allow(dead_code)]
    pub fn from_index(ifindex: u32) -> io::Result<Ifreq> {
        let mut ifr: Ifreq = unsafe { mem::uninitialized() };
        if unsafe { libc::if_indextoname(ifindex, ifr.ifr.ifr_name.as_mut_ptr()) } == ptr::null_mut() {
            return Err(io::Error::from_raw_os_error(errno()));
        }
        ifr.fd = try!(socket(&Udp::v4()));
        Ok(ifr)
    }

    unsafe fn to_i16(&self) -> i16 {
        *(self.ifr.union.as_ptr() as *const i16)
    }

    unsafe fn to_i32(&self) -> i32 {
        *(self.ifr.union.as_ptr() as *const i32)
    }

    unsafe fn to_ipaddr(&self) -> IpAddrV4 {
        let sin = &*(self.ifr.union.as_ptr() as *const sockaddr_in);
        IpAddrV4::from_bytes(mem::transmute((&*sin).sin_addr))
    }

    unsafe fn to_lladdr(&self) -> LlAddr {
        let sa = &*(self.ifr.union.as_ptr() as *const sockaddr);
        LlAddr::new(sa.sa_data[0] as u8, sa.sa_data[1] as u8, sa.sa_data[2] as u8,
                    sa.sa_data[3] as u8, sa.sa_data[4] as u8, sa.sa_data[5] as u8)
    }

    unsafe fn ioctl(&self, name: i32) -> i32 {
        libc::ioctl(self.fd, name as u64, &self.ifr)
    }

    #[allow(dead_code)]
    pub fn get_hwaddr(&self) -> io::Result<LlAddr> {
        let _ = libc_try!(self.ioctl(SIOCGIFHWADDR));
        Ok(unsafe { self.to_lladdr() })
    }

    #[allow(dead_code)]
    pub fn get_ipaddr(&self) -> io::Result<IpAddrV4> {
        let _ = libc_try!(self.ioctl(SIOCGIFADDR));
        Ok(unsafe { self.to_ipaddr() })
    }

    #[allow(dead_code)]
    pub fn get_flags(&self) -> io::Result<i16> {
        let _ = libc_try!(self.ioctl(SIOCGIFFLAGS));
        Ok(unsafe { self.to_i16() })
    }

    #[allow(dead_code)]
    pub fn get_mtu(&self) -> io::Result<i32> {
        let _ = libc_try!(self.ioctl(SIOCGIFMTU));
        Ok(unsafe { self.to_i32() })
    }

    pub fn get_index(&self) -> io::Result<u32> {
        let _ = libc_try!(self.ioctl(SIOCGIFINDEX));
        Ok(unsafe { self.to_i32() } as u32)
    }
}

impl Drop for Ifreq {
    fn drop(&mut self) {
        let _ = unsafe { libc::close(self.fd) };
    }
}

#[test]
fn test_ifreq() {
    let ifr = Ifreq::new("lo").unwrap();
    assert_eq!(ifr.get_hwaddr().unwrap(), LlAddr::new(0,0,0,0,0,0));
    assert_eq!(ifr.get_ipaddr().unwrap(), IpAddrV4::loopback());
    assert_eq!(ifr.get_mtu().unwrap(), 65536);
    assert_eq!(ifr.get_index().unwrap(), unsafe { libc::if_nametoindex(CString::new("lo").unwrap().as_ptr()) });
}
