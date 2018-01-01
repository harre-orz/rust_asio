use std::mem;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use libc;


pub trait PodTrait { }
impl PodTrait for libc::sockaddr_in { }
impl PodTrait for libc::sockaddr_in6 { }
impl PodTrait for libc::sockaddr_storage { }
#[cfg(unix)] impl PodTrait for libc::sockaddr_un { }


#[cfg(target_os = "macos")] mod bsd;
#[cfg(target_os = "macos")] pub use self::bsd::BsdSockAddr as SockAddr;


#[cfg(not(target_os = "macos"))] mod nobsd;
#[cfg(not(target_os = "macos"))] pub use self::nobsd::SockAddr;


unsafe fn memcmp<T>(lhs: *const T, rhs: *const T, len: u8) -> i32 {
    libc::memcmp(lhs as *const _, rhs as *const _, len as usize)
}


impl<T: PodTrait> PartialEq for SockAddr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.size() == other.size() && 0 == unsafe { memcmp(&self.sa, &other.sa, self.size()) }
    }
}

impl<T: PodTrait> Eq for SockAddr<T> { }

impl<T: PodTrait> PartialOrd for SockAddr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match unsafe { memcmp(&self.sa, &other.sa, self.size()) }.partial_cmp(&0) {
            Some(Ordering::Equal) => self.size().partial_cmp(&other.size()),
            cmp => cmp,
        }
    }
}

impl<T: PodTrait> Ord for SockAddr<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        match unsafe { memcmp(&self.sa, &other.sa, self.size()) }.cmp(&0) {
            Ordering::Equal => self.size().cmp(&other.size()),
            cmp => cmp,
        }
    }
}

impl<T: PodTrait> Hash for SockAddr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use std::slice;
        state.write(unsafe { slice::from_raw_parts::<u8>(mem::transmute(&self.sa), self.size() as usize) });
    }
}


impl PartialEq for SockAddr<Box<[u8]>> {
    fn eq(&self, other: &Self) -> bool {
        self.sa[..self.size() as usize].eq(&other.sa[..other.size() as usize])
    }
}

impl Eq for SockAddr<Box<[u8]>> { }

impl PartialOrd for SockAddr<Box<[u8]>> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.sa[..self.size() as usize].partial_cmp(&other.sa[..other.size() as usize])
    }
}

impl Ord for SockAddr<Box<[u8]>> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sa[..self.size() as usize].cmp(&other.sa[..other.size() as usize])
    }
}

impl Hash for SockAddr<Box<[u8]>> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.sa[..self.size() as usize])
    }
}
