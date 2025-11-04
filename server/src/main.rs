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

  let tun_device = tun_rs::DeviceBuilder::new().ipv4(tun_virtual_ipv4, shared::BASE_IPV4_MASK, None).ipv6(tun_virtual_ipv6, shared::BASE_IPV6_PREFIX).mtu(shared::MTU).build_async()?;

  // // Set up routes for the VPN subnet
  // setup_routes()?;

  vpn_service::run_vpn_service(vpn_state, udp_socket, tun_device).await?;

  Ok(())
}

// fn setup_routes() -> anyhow::Result<()> {
//   // Calculate the network address for the IPv4 subnet
//   let ipv4_network = format!("{}/{}", shared::BASE_IPV4, shared::BASE_IPV4_MASK);
//   let ipv6_network = format!("{}/{}", shared::BASE_IPV6, shared::BASE_IPV6_PREFIX);

//   // Add IPv4 route (ignore error if already exists)
//   let output = Command::new("ip").args(&["route", "add", &ipv4_network, "dev", "tun0"]).output()?;

//   if !output.status.success() {
//     let stderr = String::from_utf8_lossy(&output.stderr);
//     if !stderr.contains("File exists") {
//       return Err(anyhow::anyhow!("Failed to add IPv4 route: {}", stderr));
//     }
//     println!("IPv4 route already exists");
//   } else {
//     println!("IPv4 route added: {} via tun0", ipv4_network);
//   }

//   // Add IPv6 route (ignore error if already exists)
//   let output = Command::new("ip").args(&["-6", "route", "add", &ipv6_network, "dev", "tun0"]).output()?;

//   if !output.status.success() {
//     let stderr = String::from_utf8_lossy(&output.stderr);
//     if !stderr.contains("File exists") {
//       return Err(anyhow::anyhow!("Failed to add IPv6 route: {}", stderr));
//     }
//     println!("IPv6 route already exists");
//   } else {
//     println!("IPv6 route added: {} via tun0", ipv6_network);
//   }

//   Ok(())
// }
