use std::io;
use std::mem;
use std::marker::PhantomData;
use std::convert::From;
use std::ffi::CString;
use backbone::{RawFd, AsRawFd, AF_INET, c_void, c_char, sockaddr, sockaddr_in, close, ioctl, socket};
use IoControl;
use ip::{LlAddr, IpAddrV4, Udp};

const IFNAMSIZ: usize = 16;

/* Socket configuration controls. */
const SIOCGIFNAME: i32        = 0x8910;      /* get iface name       */
// const SIOCSIFLINK: i32        = 0x8911;      /* set iface channel        */
// const SIOCGIFCONF: i32        = 0x8912;      /* get iface list       */
const SIOCGIFFLAGS: i32       = 0x8913;      /* get flags            */
const SIOCSIFFLAGS: i32       = 0x8914;      /* set flags            */
const SIOCGIFADDR: i32        = 0x8915;      /* get PA address       */
// const SIOCSIFADDR: i32        = 0x8916;      /* set PA address       */
const SIOCGIFDSTADDR: i32     = 0x8917;      /* get remote PA address    */
// const SIOCSIFDSTADDR: i32     = 0x8918;      /* set remote PA address    */
const SIOCGIFBRDADDR: i32     = 0x8919;      /* get broadcast PA address */
// const SIOCSIFBRDADDR: i32     = 0x891a;      /* set broadcast PA address */
const SIOCGIFNETMASK: i32     = 0x891b;      /* get network PA mask      */
// const SIOCSIFNETMASK: i32     = 0x891c;      /* set network PA mask      */
// const SIOCGIFMETRIC: i32      = 0x891d;      /* get metric           */
// const SIOCSIFMETRIC: i32      = 0x891e;      /* set metric           */
// const SIOCGIFMEM: i32         = 0x891f;      /* get memory address (BSD) */
// const SIOCSIFMEM: i32         = 0x8920;      /* set memory address (BSD) */
const SIOCGIFMTU: i32         = 0x8921;      /* get MTU size         */
const SIOCSIFMTU: i32         = 0x8922;      /* set MTU size         */
// const SIOCSIFNAME: i32        = 0x8923;      /* set interface name       */
const SIOCSIFHWADDR: i32      = 0x8924;      /* set hardware address     */
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

pub trait IfreqName : Sized {
    fn name() -> i32;
}

#[repr(C)]
struct ifreq {
    ifr_name: [c_char; IFNAMSIZ],
    union: [u8; 24],
}

/// IO control command to interface request data.
pub struct Ifreq<T> {
    ifr: ifreq,
    marker: PhantomData<T>,
}

impl<T> Ifreq<T> {
    pub fn new<U: Into<Vec<u8>>>(name: U) -> io::Result<Ifreq<T>> {
        match CString::new(name.into()) {
            Ok(s) =>  {
                let bytes = s.as_bytes();
                if bytes.len() >= IFNAMSIZ {
                    return Err(io::Error::new(io::ErrorKind::Other, "too long name"));
                }
                let mut ifr: ifreq = unsafe { mem::zeroed() };
                ifr.ifr_name[..bytes.len()].copy_from_slice(unsafe { mem::transmute(bytes) });
                Ok(Ifreq {
                    ifr: ifr,
                    marker: PhantomData,
                })
            },
            Err(err) => Err(io::Error::from(err)),
        }
    }

    pub fn from_index(index: u32) -> io::Result<Ifreq<T>> {
        let soc = try!(IfreqSocket::new());
        let mut ifr: Ifreq<IfreqGetNameT> = Ifreq {
            ifr: unsafe { mem::zeroed() },
            marker: PhantomData,
        };
        ifr.set_index(index);
        try!(soc.io_control(&mut ifr));
        Ok(ifr.into())
    }

    pub fn into<U>(self) -> Ifreq<U> {
        Ifreq {
            ifr: self.ifr,
            marker: PhantomData,
        }
    }

    fn get_i16(&self) -> i16 {
        unsafe { *(self.ifr.union.as_ptr() as *const i16) }
    }

    fn set_i16(&mut self, data: i16) {
        unsafe { *(self.ifr.union.as_mut_ptr() as *mut i16) = data }
    }

    fn get_i32(&self) -> i32 {
        unsafe { *(self.ifr.union.as_ptr() as *const i32) }
    }

    fn set_i32(&mut self, data: i32) {
        unsafe { *(self.ifr.union.as_mut_ptr() as *mut i32) = data }
    }

    fn _get_hwaddr(&self) -> LlAddr {
        let sa: &sockaddr = unsafe { mem::transmute(&self.ifr.union) };
        LlAddr::from_bytes(unsafe { *(sa.sa_data.as_ptr() as *const [u8; 6]) })
    }

    fn _set_hwaddr(&mut self, data: LlAddr) {
        let sa: &mut sockaddr = unsafe { mem::transmute(&mut self.ifr.union) };
        sa.sa_family = 0x0304u16.to_be();
        unsafe { *(sa.sa_data.as_mut_ptr() as *mut [u8; 6]) = *data.as_bytes() };
    }

    fn _get_ipaddr(&self) -> IpAddrV4 {
        let sin: &sockaddr_in = unsafe { mem::transmute(&self.ifr.union) };
        IpAddrV4::from_bytes(unsafe { mem::transmute(sin.sin_addr) })
    }

    fn _set_ipaddr(&mut self, data: IpAddrV4) {
        let sin: &mut sockaddr_in = unsafe { mem::transmute(&mut self.ifr.union) };
        sin.sin_family = AF_INET as u16;
        sin.sin_addr = unsafe { mem::transmute(data) };
    }
}

impl<T: IfreqName> IoControl for Ifreq<T> {
    type Data = c_void;

    fn name(&self) -> i32 {
        T::name()
    }

    fn data(&mut self) -> &mut Self::Data {
        unsafe { mem::transmute(&mut self.ifr) }
    }
}


#[doc(hidden)]
pub struct IfreqGetIndexT;

impl IfreqName for IfreqGetIndexT {
    fn name() -> i32 { SIOCGIFINDEX }
}

impl Ifreq<IfreqGetIndexT> {
    pub fn get_index(&self) -> u32 {
        self.get_i32() as u32
    }
}

pub type IfreqGetIndex = Ifreq<IfreqGetIndexT>;

struct IfreqGetNameT;

impl IfreqName for IfreqGetNameT {
    fn name() -> i32 { SIOCGIFNAME }
}

impl Ifreq<IfreqGetNameT> {
    pub fn set_index(&mut self, data: u32) {
        self.set_i32(data as i32)
    }
}


#[doc(hidden)]
pub struct IfreqGetFlagsT;

impl IfreqName for IfreqGetFlagsT {
    fn name() -> i32 { SIOCGIFFLAGS }
}

impl Ifreq<IfreqGetFlagsT> {
    pub fn get_flags(&self) -> i16 {
        self.get_i16()
    }
}

pub type IfreqGetFlags = Ifreq<IfreqGetFlagsT>;


#[doc(hidden)]
pub struct IfreqSetFlagsT;

impl IfreqName for IfreqSetFlagsT {
    fn name() -> i32 { SIOCSIFFLAGS }
}

impl Ifreq<IfreqSetFlagsT> {
    pub fn set_flags(&mut self, flags: i16) {
        self.set_i16(flags)
    }
}

pub type IfreqSetFlags = Ifreq<IfreqSetFlagsT>;


#[doc(hidden)]
pub struct IfreqGetMTUSizeT;

impl IfreqName for IfreqGetMTUSizeT {
    fn name() -> i32 { SIOCGIFMTU }
}

impl Ifreq<IfreqGetMTUSizeT> {
    pub fn get_mtu_size(&self) -> i32 {
        self.get_i32()
    }
}

pub type IfreqGetMTUSize = Ifreq<IfreqGetMTUSizeT>;


#[doc(hidden)]
pub struct IfreqSetMTUSizeT;

impl IfreqName for IfreqSetMTUSizeT {
    fn name() -> i32 { SIOCSIFMTU }
}

impl Ifreq<IfreqSetMTUSizeT> {
    pub fn set_mtu_size(&mut self, data: i32) {
        self.set_i32(data)
    }
}

pub type IfreqSetMTUSize = Ifreq<IfreqSetMTUSizeT>;


#[doc(hidden)]
pub struct IfreqGetHwAddrT;

impl IfreqName for IfreqGetHwAddrT {
    fn name() -> i32 { SIOCGIFHWADDR }
}

impl Ifreq<IfreqGetHwAddrT> {
    pub fn get_hwaddr(&self) -> LlAddr {
        self._get_hwaddr()
    }
}

pub type IfreqGetHwAddr = Ifreq<IfreqGetHwAddrT>;


#[doc(hidden)]
pub struct IfreqSetHwAddrT;

impl IfreqName for IfreqSetHwAddrT {
    fn name() -> i32 { SIOCSIFHWADDR }
}

impl Ifreq<IfreqSetHwAddrT> {
    pub fn set_hwaddr(&mut self, data: LlAddr) {
        self._set_hwaddr(data)
    }
}

pub type IfreqSetHwAddr = Ifreq<IfreqSetHwAddrT>;


#[doc(hidden)]
pub struct IfreqAddrT;

impl IfreqName for IfreqAddrT {
    fn name() -> i32 { SIOCGIFADDR }
}

impl Ifreq<IfreqAddrT> {
    pub fn get_ipaddr(&self) -> IpAddrV4 {
        self._get_ipaddr()
    }
}

pub type IfreqAddr = Ifreq<IfreqAddrT>;


#[doc(hidden)]
pub struct IfreqNetmaskT;

impl IfreqName for IfreqNetmaskT {
    fn name() -> i32 { SIOCGIFNETMASK }
}

impl Ifreq<IfreqNetmaskT> {
    pub fn get_netmask(&self) -> IpAddrV4 {
        self._get_ipaddr()
    }
}

pub type IfreqNetmask = Ifreq<IfreqNetmaskT>;


#[doc(hidden)]
pub struct IfreqBroadcastT;

impl IfreqName for IfreqBroadcastT {
    fn name() -> i32 { SIOCGIFBRDADDR }
}

impl Ifreq<IfreqBroadcastT> {
    pub fn get_broadcast(&self) -> IpAddrV4 {
        self._get_ipaddr()
    }
}

pub type IfreqBroadcast = Ifreq<IfreqBroadcastT>;


#[doc(hidden)]
pub struct IfreqDestinateT;

impl IfreqName for IfreqDestinateT {
    fn name() -> i32 { SIOCGIFDSTADDR }
}

impl Ifreq<IfreqDestinateT> {
    pub fn get_destinate(&self) -> IpAddrV4 {
        self._get_ipaddr()
    }
}

pub type IfreqDestinate = Ifreq<IfreqDestinateT>;


/// IO control command socket for `Ifreq`.
pub struct IfreqSocket {
    fd: RawFd,
}

impl IfreqSocket {
    pub fn new() -> io::Result<IfreqSocket> {
        let fd = try!(socket(&Udp::v4()));
        Ok(IfreqSocket {
            fd: fd,
        })
    }

    pub fn io_control<T: IfreqName>(&self, cmd: &mut Ifreq<T>) -> io::Result<()> {
        ioctl(self, cmd)
    }
}

impl AsRawFd for IfreqSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for IfreqSocket {
    fn drop(&mut self) {
        close(self)
    }
}


#[test]
fn test_ifreq() {
    use libc;

    let mut ifr = IfreqGetHwAddr::new("lo").unwrap();
    let soc = IfreqSocket::new().unwrap();
    soc.io_control(&mut ifr).unwrap();
    assert_eq!(ifr.get_hwaddr(), LlAddr::new(0,0,0,0,0,0));

    let mut ifr: IfreqAddr = ifr.into();
    soc.io_control(&mut ifr).unwrap();
    assert_eq!(ifr.get_ipaddr(), IpAddrV4::loopback());

    let mut ifr: IfreqGetMTUSize = ifr.into();
    soc.io_control(&mut ifr).unwrap();
    assert_eq!(ifr.get_mtu_size(), 65536);

    let mut ifr: IfreqGetIndex = ifr.into();
    soc.io_control(&mut ifr).unwrap();
    assert_eq!(ifr.get_index(), unsafe { libc::if_nametoindex(CString::new("lo").unwrap().as_ptr()) });
}
