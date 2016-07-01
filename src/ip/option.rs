use {GetSocketOption, SetSocketOption};
use socket_base::BooleanOption;

/// Socket option for get/set an IPv6 socket supports IPv6 communication only.
#[derive(Default, Clone)]
pub struct V6Only(i32);

impl BooleanOption for V6Only {
    fn on() -> Self {
        V6Only(1)
    }

    fn is_on(&self) -> bool {
        self.0 != 0
    }
}

impl<S: IpSocket> GetSocketOption<S> for V6Only {
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

impl<S: IpSocket> SetSocketOption<S> for V6Only {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}
