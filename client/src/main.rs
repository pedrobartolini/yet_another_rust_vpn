use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

use tokio::net::UdpSocket;

mod route_setup;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let config = shared::Config::new()?;

  let mut tun_device: Option<tun_rs::AsyncDevice> = None;

  let udp_socket = UdpSocket::bind("0.0.0.0:0").await?;
  udp_socket.connect((config.server_addr, config.server_port)).await?;

  let udp_id = shared::UdpId::generate();

  let mut udp_buffer = [0u8; 4096 + 1]; // 1 byte extra for packet type
  let mut tun_buffer = [0u8; 4096];

  tun_buffer[0..4].copy_from_slice(&udp_id.as_bytes());

  udp_socket.send(udp_id.as_bytes()).await?;

  loop {
    tokio::select! {
      udp_read = udp_socket.recv(&mut udp_buffer) => handle_udp_read(udp_read, &mut udp_buffer, &mut tun_device).await?,
      tun_read = async {
        match &mut tun_device {
          Some(device) => device.recv(&mut tun_buffer[4..]).await,
          None => std::future::pending().await,
        }
      } => handle_tun_read(tun_read, &mut tun_buffer, &udp_socket).await?,
    }
  }
}

async fn handle_udp_read(udp_read: tokio::io::Result<usize>, udp_buffer: &mut [u8], tun_device: &mut Option<tun_rs::AsyncDevice>) -> anyhow::Result<()> {
  let n = udp_read.map_err(|err| anyhow::anyhow!("failed to recv udp datagram: {err:?}"))?;

  if n == 0 {
    return Err(anyhow::anyhow!("failed to recv udp datagram: EOF"))
  }

  match udp_buffer[0] {
    shared::PACKET_TYPE_FORWARD =>
      if let Some(device) = tun_device {
        device.send(&udp_buffer[1..n]).await.map_err(|err| anyhow::anyhow!("failed to send tun packet: {err:?}"))?;
      },

    shared::PACKET_TYPE_VIRTUAL_ADDRESSES => {
      let virtual_ipv4_octet: [u8; 4] = udp_buffer[1..5].try_into().unwrap();
      let virtual_ipv6_octet: [u8; 16] = udp_buffer[5..21].try_into().unwrap();

      let virtual_ipv4 = Ipv4Addr::try_from(virtual_ipv4_octet).unwrap();
      let virtual_ipv6 = Ipv6Addr::try_from(virtual_ipv6_octet).unwrap();

      // Drop the old device first if it exists
      *tun_device = None;

      let device = tun_rs::DeviceBuilder::new()
        .name("tun0")
        .ipv4(virtual_ipv4, shared::BASE_IPV4_MASK, Some(Ipv4Addr::new(virtual_ipv4.octets()[0], virtual_ipv4.octets()[1], virtual_ipv4.octets()[2], 1)))
        .ipv6(virtual_ipv6, shared::BASE_IPV6_PREFIX)
        .mtu(shared::MTU)
        .build_async()?;

      route_setup::setup_route("tun0")?;

      println!("assigned tun device");

      *tun_device = Some(device);
    }

    _ => {}
  }

  Ok(())
}

async fn handle_tun_read(tun_read: tokio::io::Result<usize>, tun_buffer: &mut [u8], udp_socket: &UdpSocket) -> anyhow::Result<()> {
  let n = tun_read.map_err(|err| anyhow::anyhow!("failed to recv tun packet: {err:?}"))?;

  if n == 0 {
    return Err(anyhow::anyhow!("failed to recv tun packet: EOF"))
  }

  udp_socket.send(&tun_buffer[..4 + n]).await.map_err(|err| anyhow::anyhow!("failed to send tun packet: {err:?}"))?;

  Ok(())
}
