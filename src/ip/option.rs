use {Protocol, GetSocketOption, SetSocketOption};
use ip::{Tcp, Udp, Icmp};
use ops::*;

pub trait IpProtocol : Protocol {}

impl IpProtocol for Tcp {}

impl IpProtocol for Udp {}

impl IpProtocol for Icmp {}


/// Socket option for get/set an IPv6 socket supports IPv6 communication only.
#[derive(Default, Clone)]
pub struct V6Only(i32);

impl V6Only {
    pub fn new(on: bool) -> V6Only {
        V6Only(on as i32)
    }

    pub fn get(&self) -> bool {
        self.0 != 0
    }

    pub fn set(&mut self, on: bool) {
        self.0 = on as i32
    }
}


impl<P: IpProtocol> GetSocketOption<P> for V6Only {
    type Data = i32;
    fn level(&self) -> i32 {
        IPPROTO_IPV6
    }

    fn name(&self) -> i32 {
        IPV6_V6ONLY
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<P: IpProtocol> SetSocketOption<P> for V6Only {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}
