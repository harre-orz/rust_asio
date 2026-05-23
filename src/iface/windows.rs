use windows_sys::Win32::NetworkManagement::IpHelper;
use windows_sys::Win32::Networking::WinSock;

pub struct Iface {}

pub struct Ifaces {}

impl Ifaces {
    pub fn new() -> Iface {
        Iface {}
    }
}
