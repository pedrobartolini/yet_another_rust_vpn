use std::process::Command;

pub fn setup_routes() -> anyhow::Result<()> {
  // Enable IP forwarding
  Command::new("sysctl").args(["-w", "net.ipv4.ip_forward=1"]).status()?;

  Command::new("sysctl").args(["-w", "net.ipv6.conf.all.forwarding=1"]).status()?;

  // NAT - use MASQUERADE on all outbound interfaces (not just eth0)
  Command::new("iptables").args(["-t", "nat", "-A", "POSTROUTING", "-s", &format!("{}/{}", shared::BASE_IPV4, shared::BASE_IPV4_MASK), "!", "-o", "tun0", "-j", "MASQUERADE"]).status()?;

  Command::new("ip6tables").args(["-t", "nat", "-A", "POSTROUTING", "-s", &format!("{}/{}", shared::BASE_IPV6, shared::BASE_IPV6_PREFIX), "!", "-o", "tun0", "-j", "MASQUERADE"]).status()?;

  Ok(())
}
