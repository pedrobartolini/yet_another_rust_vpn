use std::collections::HashSet;
use std::net::Ipv6Addr;

pub const BASE_IPV6: Ipv6Addr = Ipv6Addr::new(0xfd00, 0, 0, 1, 0, 0, 0, 0);
pub const BASE_IPV6_PREFIX: u8 = 64;

pub struct IPv6Pool {
  base:      Ipv6Addr,
  prefix:    u8,
  allocated: HashSet<Ipv6Addr>
}

impl IPv6Pool {
  pub fn new(base: Ipv6Addr, prefix: u8) -> Self {
    Self { base, prefix, allocated: Default::default() }
  }

  pub fn allocate(&mut self) -> Option<Ipv6Addr> {
    let base_u128 = u128::from(self.base);
    let host_bits = 128 - self.prefix;
    let max_hosts = 1u128 << host_bits;

    for offset in 1..max_hosts {
      let ip_u128 = base_u128 + offset;
      let ip = Ipv6Addr::from(ip_u128);

      if !self.allocated.contains(&ip) {
        self.allocated.insert(ip);
        return Some(ip);
      }
    }
    None
  }

  pub fn release(&mut self, ip: &Ipv6Addr) {
    self.allocated.remove(ip);
  }
}
