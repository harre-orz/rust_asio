use socket::*;
use socket::ip::*;
//use socket::local::*;
use ops::*;

pub trait BooleanOption : Default {
    fn on() -> Self;

    fn is_on(&self) -> bool;

    fn off() -> Self {
        Self::default()
    }

    fn is_off(&self) -> bool {
        !self.is_on()
    }
}

#[derive(Default, Clone)]
pub struct Broadcast(i32);

impl BooleanOption for Broadcast {
    fn on() -> Self {
        Broadcast(1)
    }

    fn is_on(&self) -> bool {
        self.0 != 0
    }
}

impl GetSocketOption<UdpSocket> for Broadcast {
    type Data = i32;

    fn level(&self) -> i32 {
        SOL_SOCKET
    }

    fn name(&self) -> i32 {
        SO_BROADCAST
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl SetSocketOption<UdpSocket> for Broadcast {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}

#[derive(Default, Clone)]
pub struct KeepAlive(i32);

impl BooleanOption for KeepAlive {
    fn on() -> Self {
        KeepAlive(1)
    }

    fn is_on(&self) -> bool {
        self.0 != 0
    }
}

impl<S: Socket> GetSocketOption<S> for KeepAlive {
    type Data = i32;

    fn level(&self) -> i32 {
        SOL_SOCKET
    }

    fn name(&self) -> i32 {
        SO_KEEPALIVE
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<S: Socket> SetSocketOption<S> for KeepAlive {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}

#[derive(Default, Clone)]
pub struct ReuseAddr(i32);

impl BooleanOption for ReuseAddr {
    fn on() -> Self {
        ReuseAddr(1)
    }

    fn is_on(&self) -> bool {
        self.0 != 0
    }
}

impl<S: Socket> GetSocketOption<S> for ReuseAddr {
    type Data = i32;

    fn level(&self) -> i32 {
        SOL_SOCKET
    }

    fn name(&self) -> i32 {
        SO_REUSEADDR
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

impl<S: Socket> SetSocketOption<S> for ReuseAddr {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}
