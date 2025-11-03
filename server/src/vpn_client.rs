use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;

use tokio::time::Instant;

pub struct VpnClient {
  pub sockaddr:     SocketAddr,
  pub created:      Instant,
  pub last_read:    Instant,
  pub virtual_ipv4: Ipv4Addr,
  pub virtual_ipv6: Ipv6Addr
}

impl VpnClient {
  pub fn new(sockaddr: SocketAddr, virtual_ipv4: Ipv4Addr, virtual_ipv6: Ipv6Addr, now: Instant) -> Self {
    Self { sockaddr, created: now, last_read: now, virtual_ipv4, virtual_ipv6 }
  }
}
