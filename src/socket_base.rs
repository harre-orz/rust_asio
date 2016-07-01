use {GetSocketOption, SetSocketOption};
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

impl<T> GetSocketOption<T> for Broadcast {
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

impl<T> SetSocketOption<T> for Broadcast {
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

impl<T> GetSocketOption<T> for KeepAlive {
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

impl<T> SetSocketOption<T> for KeepAlive {
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

impl<T> GetSocketOption<T> for ReuseAddr {
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

impl<T> SetSocketOption<T> for ReuseAddr {
    fn data(&self) -> &Self::Data {
        &self.0
    }
}
