use asyncio::iface::{IfaceAddrRef, Ifaces};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn main() -> Result {
    let ifaces = Ifaces::new()?;
    for iface in ifaces.iter() {
        match iface.addr() {
            IfaceAddrRef::V4(v4, len) => {
                println!("IPv4 address: {}/{} ({})", v4, len, iface.name())
            }
            IfaceAddrRef::V6(v6, len) => {
                println!("IPv6 address: {}/{} ({})", v6, len, iface.name())
            }
            IfaceAddrRef::Hw(hw) => println!("MAC address: {:?} ({})", hw.eth_addr(), iface.name()),
        }
    }
    Ok(())
}
