use std::net::IpAddr;
use std::net::SocketAddr;

use tokio::net::UdpSocket;

use super::*;

const PACKET_TYPE_FORWARD: u8 = 1;
const PACKET_TYPE_VIRTUAL_ADDRESSES: u8 = 2;

pub async fn run_vpn_service(udp_socket: UdpSocket, mut udp_state: vpn_state::VpnState, tun_device: tun_rs::AsyncDevice) -> anyhow::Result<()> {
  let mut udp_buffer = [0u8; 4096];
  let mut tun_buffer = [0u8; 4096 + 1]; // 1 byte extra for packet type prefix

  loop {
    tokio::select! {
      client_id = udp_state.next_timeout() => udp_state.remove_client(&client_id),
      udp_read = udp_socket.recv_from(&mut udp_buffer) => handle_udp_read(udp_read,  &mut udp_buffer, &mut udp_state, &udp_socket, &tun_device).await?,
      tun_read = tun_device.recv(&mut tun_buffer) => handle_tun_read(tun_read, &mut tun_buffer[1..], &mut udp_state, &udp_socket).await?,
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
  if n < 5 {
    return Ok(());
  }

  let client_id = shared::UdpId::from([udp_buffer[0], udp_buffer[1], udp_buffer[2], udp_buffer[3]]);
  let is_new_client = udp_state.add_client(&client_id, sockaddr)?;

  match udp_buffer[4] >> 4 {
    4 => {
      let Ok(mut ipv4_packet) = smoltcp::wire::Ipv4Packet::new_checked(&mut udp_buffer[4..]) else { return Ok(()) };
      ipv4_packet.set_src_addr(*udp_state.get_client_virtual_ipv4(&client_id).unwrap());
      ipv4_packet.fill_checksum();
    }
    6 => {
      let Ok(mut ipv6_packet) = smoltcp::wire::Ipv6Packet::new_checked(&mut udp_buffer[4..]) else { return Ok(()) };
      ipv6_packet.set_src_addr(*udp_state.get_client_virtual_ipv6(&client_id).unwrap());
    }
    _ => return Ok(())
  }

  tun_device.send(&udp_buffer[4..n]).await.map_err(|err| anyhow::anyhow!("failed to send udp datagram: {err:?}"))?;

  if is_new_client {
    let mut out_buffer = [0u8; 1 + 4 + 16];

    out_buffer[0] = PACKET_TYPE_VIRTUAL_ADDRESSES;

    // Write IPv4 (4 bytes)
    out_buffer[1..5].copy_from_slice(&udp_state.get_client_virtual_ipv4(&client_id).unwrap().octets());

    // Write IPv6 (16 bytes)
    out_buffer[5..21].copy_from_slice(&udp_state.get_client_virtual_ipv6(&client_id).unwrap().octets());

    udp_socket.send_to(&out_buffer, sockaddr).await.map_err(|err| anyhow::anyhow!("failed to send tun packet: {err:?}"))?;
  }

  Ok(())
}

async fn handle_tun_read(tun_read: tokio::io::Result<usize>, tun_buffer: &mut [u8], udp_state: &mut vpn_state::VpnState, udp_socket: &UdpSocket) -> anyhow::Result<()> {
  let n = tun_read.map_err(|err| anyhow::anyhow!("failed to recv tun packet: {err:?}"))?;

  if n == 0 {
    return Err(anyhow::anyhow!("failed to recv tun packet: EOF"))
  }

  let virtual_ip = match tun_buffer[1] >> 4 {
    4 => smoltcp::wire::Ipv4Packet::new_checked(&tun_buffer).ok().map(|packet| IpAddr::V4(packet.dst_addr())),
    6 => smoltcp::wire::Ipv6Packet::new_checked(&tun_buffer).ok().map(|packet| IpAddr::V6(packet.dst_addr())),
    _ => return Ok(())
  };

  let Some(virtual_ip) = virtual_ip else { return Ok(()) };
  let Some(sockaddr) = udp_state.get_sockaddr(&virtual_ip) else { return Ok(()) };

  tun_buffer[0] = PACKET_TYPE_FORWARD; // Type: packet

  udp_socket.send_to(&tun_buffer[..n + 1], sockaddr).await.map_err(|err| anyhow::anyhow!("failed to send tun packet: {err:?}"))?;

  Ok(())
}
