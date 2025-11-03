use std::collections::HashSet;
use std::net::Ipv4Addr;

pub struct IPv4Pool {
  base:      Ipv4Addr,
  mask:      u8,
  allocated: HashSet<Ipv4Addr>
}

impl IPv4Pool {
  pub fn new(base: Ipv4Addr, mask: u8) -> Self {
    Self { base, mask, allocated: Default::default() }
  }

  pub fn allocate(&mut self) -> Option<Ipv4Addr> {
    let start = u32::from(self.base) + 1;
    let end = start + (1 << (32 - self.mask)) - 2;

    for ip_u32 in start..=end {
      let ip = Ipv4Addr::from(ip_u32);

      if !self.allocated.contains(&ip) {
        self.allocated.insert(ip);
        return Some(ip);
      }
    }
    None
  }

  pub fn release(&mut self, ip: &Ipv4Addr) {
    self.allocated.remove(ip);
  }
}
