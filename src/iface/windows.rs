use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::NetworkManagement::IpHelper;

pub struct Iface {}

pub struct Ifaces {
}

impl Ifaces {
    pub fn new() -> Iface {
        Iface {}
    }
}