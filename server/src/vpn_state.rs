use std::collections::HashMap;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::time::Instant;

use super::*;

pub struct VpnState {
  clients:      HashMap<shared::UdpId, vpn_client::VpnClient>,
  ip_indexer:   HashMap<IpAddr, shared::UdpId>,
  next_timeout: Option<(shared::UdpId, Instant)>,

  ipv6_pool: shared::IPv6Pool,
  ipv4_pool: shared::IPv4Pool
}

impl VpnState {
  pub fn new(ipv4_pool: shared::IPv4Pool, ipv6_pool: shared::IPv6Pool) -> Self {
    Self { clients: Default::default(), ip_indexer: Default::default(), next_timeout: Default::default(), ipv4_pool, ipv6_pool }
  }

  pub fn add_client(&mut self, client_id: &shared::UdpId, sockaddr: SocketAddr) -> anyhow::Result<bool> {
    let now = tokio::time::Instant::now();

    let mut is_new_client = false;

    match self.clients.get_mut(client_id) {
      Some(client) => {
        client.sockaddr = sockaddr;
        client.last_read = now;
      }
      None => {
        let Some(virtual_ipv4) = self.ipv4_pool.allocate() else {
          return Err(anyhow::anyhow!("could not allocate virtual ipv4"));
        };

        let Some(virtual_ipv6) = self.ipv6_pool.allocate() else {
          self.ipv4_pool.release(&virtual_ipv4);
          return Err(anyhow::anyhow!("could not allocate virtual ipv6"));
        };

        self.clients.insert(*client_id, vpn_client::VpnClient::new(sockaddr, virtual_ipv4, virtual_ipv6, now));
        self.ip_indexer.insert(IpAddr::V4(virtual_ipv4), *client_id);
        self.ip_indexer.insert(IpAddr::V6(virtual_ipv6), *client_id);

        is_new_client = true;
      }
    }

    if self.next_timeout.as_ref().is_none_or(|(id, instant)| id == client_id || *instant > now) {
      self.recalculate_next_timeout();
    }

    Ok(is_new_client)
  }

  pub fn get_client_virtual_ipv4(&self, client_id: &shared::UdpId) -> Option<&Ipv4Addr> {
    self.clients.get(client_id).map(|client| &client.virtual_ipv4)
  }

  pub fn get_client_virtual_ipv6(&self, client_id: &shared::UdpId) -> Option<&Ipv6Addr> {
    self.clients.get(client_id).map(|client| &client.virtual_ipv6)
  }

  pub fn remove_client(&mut self, client_id: &shared::UdpId) {
    if let Some(client) = self.clients.remove(client_id) {
      let virtual_ipv4 = client.virtual_ipv4;
      let virtual_ipv6 = client.virtual_ipv6;

      self.ipv4_pool.release(&virtual_ipv4);
      self.ipv6_pool.release(&virtual_ipv6);

      self.ip_indexer.remove(&IpAddr::V4(virtual_ipv4));
      self.ip_indexer.remove(&IpAddr::V6(virtual_ipv6));
    }

    if self.next_timeout.as_ref().is_none_or(|(next_timeout_client_id, _)| next_timeout_client_id == client_id) {
      self.recalculate_next_timeout();
    }
  }

  pub fn get_sockaddr(&self, virtual_ip: &IpAddr) -> Option<SocketAddr> {
    let client_id = self.ip_indexer.get(&virtual_ip)?;
    let client = self.clients.get(client_id)?;
    Some(client.sockaddr)
  }

  pub fn next_timeout(&self) -> impl Future<Output = shared::UdpId> {
    let next_timeout = self.next_timeout.clone();

    const CLIENT_TIMEOUT_DURATION: Duration = Duration::from_secs(30);

    async move {
      match next_timeout {
        Some((next_timeout_client_id, next_timeout_instant)) => {
          tokio::time::sleep_until(next_timeout_instant + CLIENT_TIMEOUT_DURATION).await;
          return next_timeout_client_id
        }
        None => std::future::pending().await
      }
    }
  }

  fn recalculate_next_timeout(&mut self) {
    self.next_timeout = self.clients.iter().min_by_key(|(_, client)| client.last_read).map(|(id, client)| (*id, client.last_read));
  }
}
