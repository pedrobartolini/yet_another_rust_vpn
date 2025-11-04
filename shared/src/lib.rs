mod config;
mod ipv4_pool;
mod ipv6_pool;
mod udp_id;

pub use config::Config;
pub use ipv4_pool::*;
pub use ipv6_pool::*;
pub use udp_id::UdpId;

pub const MTU: u16 = 1400;

pub const PACKET_TYPE_FORWARD: u8 = 1;
pub const PACKET_TYPE_VIRTUAL_ADDRESSES: u8 = 2;
