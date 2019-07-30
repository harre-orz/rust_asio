//
// Copyrighy (c) 2016-2019 Haruhiko Uchida
// The software is released under the MIT license. see LICENSE.txt
// https://github.com/harre-orz/rust_asio/blob/master/LICENSE.txt

//!
//! The `asyio` is ASYnchronous I/O library.
//!
//! C++ Boost Libraryにインタフェースは似ていますが、コールバックではなくコルーチンで実装しています。
//!

extern crate context as context_;
extern crate libc as libc_;

mod libc;
mod error {
    //--unix--//
    #[cfg(unix)]
    mod unix;
    #[cfg(unix)]
    pub use self::unix::*;
}
mod executor {
    mod mutex;

    //--linux--//
    #[cfg(target_os = "linux")]
    mod epoll;
    #[cfg(target_os = "linux")]
    mod timerfd;
    #[cfg(target_os = "linux")]
    pub use self::epoll::{Reactor, ReactorCallback};
    #[cfg(target_os = "linux")]
    pub use self::timerfd::Interrupter;

    //--all--//
    mod context;
    pub use self::context::{AsIoContext, IoContext, Ready, SocketContext, YieldContext, TimerQueue};
}
mod socket {
    //--unix--//
    #[cfg(unix)]
    mod unix;
    #[cfg(unix)]
    pub use self::unix::*;

    mod ops;
    pub use self::ops::{
        bk_accept, bk_connect, bk_read_some, bk_receive, bk_receive_from, bk_send, bk_send_to,
        bk_write_some, nb_accept, nb_connect, nb_read_some, nb_receive, nb_receive_from, nb_send,
        nb_send_to, nb_write_some,
    };
}
pub mod socket_base;
pub mod local {
    mod dgram;
    mod endpoint;
    mod pair;
    mod seq_packet;
    mod stream;
    pub use self::dgram::{LocalDgram, LocalDgramEndpoint, LocalDgramSocket};
    pub use self::endpoint::LocalEndpoint;
    pub use self::pair::LocalPair;
    pub use self::seq_packet::{
        LocalSeqPacket, LocalSeqPacketEndpoint, LocalSeqPacketListener, LocalSeqPacketSocket,
    };
    pub use self::stream::{
        LocalStream, LocalStreamEndpoint, LocalStreamListener, LocalStreamSocket,
    };
}
pub mod ip {
    mod addr;
    mod addr_v4;
    mod addr_v6;
    mod endpoint;
    mod from_str;
    mod icmp;
    mod iface;
    mod options;
    mod resolver;
    mod tcp;
    mod udp;
    pub use self::addr::{IpAddr, LlAddr};
    pub use self::addr_v4::IpAddrV4;
    pub use self::addr_v6::IpAddrV6;
    pub use self::endpoint::IpEndpoint;
    pub use self::icmp::{Icmp, IcmpEndpoint, IcmpResolver, IcmpSocket};
    pub use self::iface::Iface;
    pub use self::options::{
        host_name, MulticastEnableLoopback, MulticastHops, MulticastJoinGroup, MulticastLeaveGroup,
        NoDelay, OutboundInterface, UnicastHops, V6Only,
    };
    pub use self::resolver::{Resolver, ResolverIter, ResolverQuery};
    pub use self::tcp::{Tcp, TcpEndpoint, TcpListener, TcpResolver, TcpSocket};
    pub use self::udp::{Udp, UdpEndpoint, UdpResolver, UdpSocket};
}
pub mod generic {
    mod dgram;
    mod endpoint;
    mod raw;
    mod seq_packet;
    mod stream;
    pub use self::dgram::{GenericDgram, GenericDgramEndpoint, GenericDgramSocket};
    pub use self::endpoint::GenericEndpoint;
    pub use self::raw::{GenericRaw, GenericRawEndpoint, GenericRawSocket};
    pub use self::seq_packet::{
        GenericSeqPacket, GenericSeqPacketEndpoint, GenericSeqPacketListener,
        GenericSeqPacketSocket,
    };
    pub use self::stream::{
        GenericStream, GenericStreamEndpoint, GenericStreamListener, GenericStreamSocket,
    };
}
mod dgram_socket;
mod socket_listener;
mod stream_socket;

pub use self::dgram_socket::DgramSocket;
pub use self::executor::{AsIoContext, IoContext, YieldContext};
pub use self::socket_listener::SocketListener;
pub use self::stream_socket::StreamSocket;
