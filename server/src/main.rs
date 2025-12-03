use tokio::net::UdpSocket;

mod vpn_client;
mod vpn_service;
mod vpn_state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let config = shared::Config::new_from_embed()?;

  let mut ipv4_pool = shared::IPv4Pool::new(shared::BASE_IPV4, shared::BASE_IPV4_MASK);
  let mut ipv6_pool = shared::IPv6Pool::new(shared::BASE_IPV6, shared::BASE_IPV6_PREFIX);

  let tun_virtual_ipv4 = ipv4_pool.allocate().ok_or(anyhow::anyhow!("could not allocate local tun virtual ipv4"))?;
  let tun_virtual_ipv6 = ipv6_pool.allocate().ok_or(anyhow::anyhow!("could not allocate local tun virtual ipv6"))?;

  let vpn_state = vpn_state::VpnState::new(ipv4_pool, ipv6_pool);

  let udp_socket = UdpSocket::bind(("0.0.0.0", config.server_port)).await?;

  let tun_device = tun_rs::DeviceBuilder::new().name("tun0").ipv4(tun_virtual_ipv4, shared::BASE_IPV4_MASK, None).ipv6(tun_virtual_ipv6, shared::BASE_IPV6_PREFIX).mtu(shared::MTU).build_async()?;

  // Set up routes for the VPN subnet
  setup_routes()?;

  vpn_service::run_vpn_service(vpn_state, udp_socket, tun_device).await?;

  Ok(())
}

use std::process::Command;

fn setup_routes() -> anyhow::Result<()> {
  // Enable IP forwarding
  Command::new("sysctl").args(["-w", "net.ipv4. ip_forward=1"]).status()?;

  Command::new("sysctl").args(["-w", "net.ipv6.conf.all.forwarding=1"]).status()?;

  // Add routes for VPN subnets through tun0
  Command::new("ip").args(["route", "add", &format!("{}/{}", shared::BASE_IPV4, shared::BASE_IPV4_MASK), "dev", "tun0"]).status()?;

  Command::new("ip").args(["-6", "route", "add", &format!("{}/{}", shared::BASE_IPV6, shared::BASE_IPV6_PREFIX), "dev", "tun0"]).status()?;

  // Enable NAT for VPN traffic (optional, if clients need internet access)
  Command::new("iptables").args(["-t", "nat", "-A", "POSTROUTING", "-s", &format!("{}/{}", shared::BASE_IPV4, shared::BASE_IPV4_MASK), "-o", "eth0", "-j", "MASQUERADE"]).status()?;

  Command::new("ip6tables").args(["-t", "nat", "-A", "POSTROUTING", "-s", &format!("{}/{}", shared::BASE_IPV6, shared::BASE_IPV6_PREFIX), "-o", "eth0", "-j", "MASQUERADE"]).status()?;

  Ok(())
}
