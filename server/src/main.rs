use tokio::net::UdpSocket;

mod vpn_client;
mod vpn_service;
mod vpn_state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let config = shared::Config::new()?;

  let mut ipv4_pool = shared::IPv4Pool::new(shared::BASE_IPV4, shared::BASE_IPV4_MASK);
  let mut ipv6_pool = shared::IPv6Pool::new(shared::BASE_IPV6, shared::BASE_IPV6_PREFIX);

  let tun_virtual_ipv4 = ipv4_pool.allocate().ok_or(anyhow::anyhow!("could not allocate local tun virtual ipv4"))?;
  let tun_virtual_ipv6 = ipv6_pool.allocate().ok_or(anyhow::anyhow!("could not allocate local tun virtual ipv6"))?;

  let vpn_state = vpn_state::VpnState::new(ipv4_pool, ipv6_pool);

  let udp_socket = UdpSocket::bind(("0.0.0.0", config.server_port)).await?;

  let tun_device = tun_rs::DeviceBuilder::new().name("tun0").ipv4(tun_virtual_ipv4, shared::BASE_IPV4_MASK, None).ipv6(tun_virtual_ipv6, shared::BASE_IPV6_PREFIX).mtu(shared::MTU).build_async()?;

  vpn_service::run_vpn_service(udp_socket, vpn_state, tun_device).await?;

  Ok(())
}
