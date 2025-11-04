use tokio::net::UdpSocket;

#[tokio::main]
async fn main() {}

async fn run_vpn_service(udp_socket: UdpSocket) -> anyhow::Result<()> {
  let config = shared::Config::new()?;

  let tun_device = tun_rs::DeviceBuilder::new().name("tun0").ipv4(shared::BASE_IPV4, shared::BASE_IPV4_MASK, None).ipv6(shared::BASE_IPV6, shared::BASE_IPV6_PREFIX).mtu(shared::MTU).build_async()?;

  let udp_socket = UdpSocket::bind((config.server_addr, config.server_port)).await?;

  let mut udp_buffer = [0u8; 4096 + 1]; // 1 byte extra for packet type
  let mut tun_buffer = [0u8; 4096];

  loop {
    tokio::select! {
      udp_read = udp_socket.recv(&mut udp_buffer) => handle_udp_read(udp_read,  &mut udp_buffer, &tun_device).await?,
      tun_read = tun_device.recv(&mut tun_buffer) => handle_tun_read(tun_read, &mut tun_buffer, &udp_socket).await?,
    }
  }
}

async fn handle_udp_read(udp_read: tokio::io::Result<usize>, udp_buffer: &mut [u8], tun_device: &tun_rs::AsyncDevice) -> anyhow::Result<()> {
  let n = udp_read.map_err(|err| anyhow::anyhow!("failed to recv udp datagram: {err:?}"))?;

  if n == 0 {
    return Err(anyhow::anyhow!("failed to recv udp datagram: EOF"))
  }

  match udp_buffer[0] {
    shared::PACKET_TYPE_FORWARD => {
      tun_device.send(&udp_buffer[..n]).await.map_err(|err| anyhow::anyhow!("failed to send tun packet: {err:?}"))?;
    }

    shared::PACKET_TYPE_VIRTUAL_ADDRESSES => {}

    _ => {}
  }

  Ok(())
}

async fn handle_tun_read(tun_read: tokio::io::Result<usize>, tun_buffer: &mut [u8], udp_socket: &UdpSocket) -> anyhow::Result<()> {
  let n = tun_read.map_err(|err| anyhow::anyhow!("failed to recv tun packet: {err:?}"))?;

  if n == 0 {
    return Err(anyhow::anyhow!("failed to recv tun packet: EOF"))
  }

  udp_socket.send(&tun_buffer[..n]).await.map_err(|err| anyhow::anyhow!("failed to send tun packet: {err:?}"))?;

  Ok(())
}
