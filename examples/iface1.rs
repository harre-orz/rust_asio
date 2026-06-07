use asyncio::iface::{IfaceAddrRef, Ifaces};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn main() -> Result {
    for iface in Ifaces::new()?.iter() {
        match iface.addr() {
            IfaceAddrRef::V4(v4, prefix_len) => {
                println!("IPv4 address: {}/{} ({})", v4, prefix_len, iface.name())
            }
            IfaceAddrRef::V6(v6, prefix_len, _) => {
                println!("IPv6 address: {}/{} ({})", v6, prefix_len, iface.name())
            }
            IfaceAddrRef::Hw(hw) => {
                println!("MAC address: {:?} ({})", hw.eth_addr(), iface.name())
            }
        }
    }
    Ok(())
}
