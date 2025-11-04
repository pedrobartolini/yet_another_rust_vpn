use std::net::IpAddr;
use std::net::SocketAddr;

use tokio::net::UdpSocket;

use super::*;

pub async fn run_vpn_service(mut vpn_state: vpn_state::VpnState, udp_socket: UdpSocket, tun_device: tun_rs::AsyncDevice) -> anyhow::Result<()> {
  let mut udp_buffer = [0u8; 4096];
  let mut tun_buffer = [0u8; 4096 + 1]; // 1 byte extra for packet type prefix

  loop {
    tokio::select! {
      client_id = vpn_state.next_timeout() => vpn_state.remove_client(&client_id),
      udp_read = udp_socket.recv_from(&mut udp_buffer) => handle_udp_read(udp_read,  &mut udp_buffer, &mut vpn_state, &udp_socket, &tun_device).await?,
      tun_read = tun_device.recv(&mut tun_buffer[1..]) => handle_tun_read(tun_read, &mut tun_buffer, &mut vpn_state, &udp_socket).await?,
    };
  }
}

async fn handle_udp_read(
  udp_read: tokio::io::Result<(usize, SocketAddr)>,
  udp_buffer: &mut [u8],
  udp_state: &mut vpn_state::VpnState,
  udp_socket: &UdpSocket,
  tun_device: &tun_rs::AsyncDevice
) -> anyhow::Result<()> {
  let (n, sockaddr) = udp_read.map_err(|err| anyhow::anyhow!("failed to recv udp datagram: {err:?}"))?;

  if n == 0 {
    return Err(anyhow::anyhow!("failed to recv udp datagram: EOF"))
  }

  // min valid datagram
  if n < 4 {
    return Ok(());
  }

  let client_id = shared::UdpId::from([udp_buffer[0], udp_buffer[1], udp_buffer[2], udp_buffer[3]]);
  let is_new_client = udp_state.add_client(&client_id, sockaddr)?;

  if is_new_client {
    let mut out_buffer = [0u8; 1 + 4 + 16];

    out_buffer[0] = shared::PACKET_TYPE_VIRTUAL_ADDRESSES;

    // Write IPv4 (4 bytes)
    out_buffer[1..5].copy_from_slice(&udp_state.get_client_virtual_ipv4(&client_id).unwrap().octets());

    // Write IPv6 (16 bytes)
    out_buffer[5..21].copy_from_slice(&udp_state.get_client_virtual_ipv6(&client_id).unwrap().octets());

    udp_socket.send_to(&out_buffer, sockaddr).await.map_err(|err| anyhow::anyhow!("failed to send tun packet: {err:?}"))?;
  }

  if n < 24 {
    return Ok(())
  }

  match udp_buffer[4] >> 4 {
    4 => {
      let mut ipv4_packet = smoltcp::wire::Ipv4Packet::new_checked(&mut udp_buffer[4..]).map_err(|err| anyhow::anyhow!("failed to parse ipv4 packet {err:?}"))?;
      ipv4_packet.set_src_addr(*udp_state.get_client_virtual_ipv4(&client_id).unwrap());
      ipv4_packet.fill_checksum();
    }
    6 => {
      let mut ipv6_packet = smoltcp::wire::Ipv6Packet::new_checked(&mut udp_buffer[4..]).map_err(|err| anyhow::anyhow!("failed to parse ipv6 packet {err:?}"))?;
      ipv6_packet.set_src_addr(*udp_state.get_client_virtual_ipv6(&client_id).unwrap());
    }
    _ => return Ok(())
  }

  tun_device.send(&udp_buffer[4..n + 4]).await.map_err(|err| anyhow::anyhow!("failed to send udp datagram: {err:?}"))?;

  Ok(())
}

async fn handle_tun_read(tun_read: tokio::io::Result<usize>, tun_buffer: &mut [u8], udp_state: &mut vpn_state::VpnState, udp_socket: &UdpSocket) -> anyhow::Result<()> {
  let n = tun_read.map_err(|err| anyhow::anyhow!("failed to recv tun packet: {err:?}"))?;

  if n == 0 {
    return Err(anyhow::anyhow!("failed to recv tun packet: EOF"))
  }

  let ip_version = tun_buffer[1] >> 4;

  let virtual_ip = match ip_version {
    4 => smoltcp::wire::Ipv4Packet::new_checked(&tun_buffer[1..]).ok().map(|packet| IpAddr::V4(packet.dst_addr())),
    6 => smoltcp::wire::Ipv6Packet::new_checked(&tun_buffer[1..]).ok().map(|packet| IpAddr::V6(packet.dst_addr())),
    _ => return Ok(())
  };

  let Some(virtual_ip) = virtual_ip else { return Ok(()) };
  let Some(sockaddr) = udp_state.get_sockaddr(&virtual_ip) else { return Ok(()) };

  tun_buffer[0] = shared::PACKET_TYPE_FORWARD; // Type: packet

  udp_socket.send_to(&tun_buffer[..n + 1], sockaddr).await.map_err(|err| anyhow::anyhow!("failed to send tun packet: {err:?}"))?;

  Ok(())
}
