use std::slice;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use libc::memcmp;

pub trait PODTrait { }

impl<T: PODTrait> Eq for SockAddrImpl<T> { }

impl<T: PODTrait> PartialEq for SockAddrImpl<T> {
    fn eq(&self, other: &Self) -> bool {
        self.size() == other.size() && unsafe {
            memcmp(
                self.data() as *const _,
                other.data() as *const _,
                self.size()
            )
        } == 0
    }
}

impl<T: PODTrait> Ord for SockAddrImpl<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        match unsafe {
            memcmp(
                self.data() as *const _,
                other.data() as *const _,
                self.size()
            )
        }.cmp(&0){
            Ordering::Equal =>
                self.size().cmp(&other.size()),
            cmp => cmp,
        }
    }
}

impl<T: PODTrait> PartialOrd for SockAddrImpl<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match unsafe {
            memcmp(
                self.data() as *const _,
                other.data() as *const _,
                self.size()
            )
        }.partial_cmp(&0){
            Some(Ordering::Equal) =>
                self.size().partial_cmp(&other.size()),
            cmp => cmp,
        }
    }
}

impl<T: PODTrait> Hash for SockAddrImpl<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(unsafe { slice::from_raw_parts(
            self.data() as *const _,
            self.size()
        ) });
        state.write_usize(self.size());
    }
}

impl<T: PODTrait> Deref for SockAddrImpl<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.data() as *const _) }
    }
}

impl<T: PODTrait> DerefMut for SockAddrImpl<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.data() as *mut _) }
    }
}

impl Eq for SockAddrImpl<Box<[u8]>> { }

impl PartialEq for SockAddrImpl<Box<[u8]>> {
    fn eq(&self, other: &Self) -> bool {
        self.size() == other.size() && unsafe {
            memcmp(
                self.data() as *const _,
                other.data() as *const _,
                self.size()
            )
        } == 0
    }
}

impl Ord for SockAddrImpl<Box<[u8]>> {
    fn cmp(&self, other: &Self) -> Ordering {
        match unsafe {
            memcmp(
                self.data() as *const _,
                other.data( ) as *const _,
                self.size()
            )
        }.cmp(&0){
            Ordering::Equal =>
                self.size().cmp(&other.size()),
            cmp => cmp,
        }
    }
}

impl PartialOrd for SockAddrImpl<Box<[u8]>> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match unsafe {
            memcmp(
                self.data() as *const _,
                other.data() as *const _,
                self.size()
            )
        }.partial_cmp(&0){
            Some(Ordering::Equal) =>
                self.size().partial_cmp(&other.size()),
            cmp => cmp,
        }
    }
}

impl Hash for SockAddrImpl<Box<[u8]>> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(unsafe { slice::from_raw_parts(
            self.data() as *const _,
            self.size()
        ) });
        state.write_usize(self.size());
    }
}

impl Deref for SockAddrImpl<Box<[u8]>> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.data() as *const _,self.size()) }
    }
}

impl DerefMut for SockAddrImpl<Box<[u8]>> {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.data() as *mut _, self.size()) }
    }
}

#[cfg(target_os = "macos")] mod bsd;
#[cfg(target_os = "macos")] pub use self::bsd::BSDSockAddrImpl as SockAddrImpl;

#[cfg(not(target_os = "macos"))] mod nobsd;
#[cfg(not(target_os = "macos"))] pub use self::nobsd::SockAddrImpl;
