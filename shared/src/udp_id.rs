use std::hash::Hash;
use std::hash::Hasher;

const KEY: &[u8] = include_bytes!("../../env/udp_key.bin");

pub const UDP_ID_LEN: usize = 4;
const CHECKSUM_LEN: usize = 2;
pub const UDP_ID_TOTAL_LEN: usize = UDP_ID_LEN + CHECKSUM_LEN;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UdpId {
  id:       [u8; UDP_ID_LEN],
  checksum: [u8; CHECKSUM_LEN]
}

impl UdpId {
  #[inline]
  fn compute_checksum(id: &[u8; UDP_ID_LEN]) -> [u8; CHECKSUM_LEN] {
    let mut sum: u16 = 0;
    for (i, &byte) in id.iter().enumerate() {
      sum = sum.wrapping_add((byte ^ KEY[i % KEY.len()]) as u16);
      sum = sum.rotate_left(3);
    }
    sum ^= u16::from_le_bytes([KEY[0], KEY[KEY.len() - 1]]);
    sum.to_le_bytes()
  }

  #[inline]
  pub fn generate() -> Self {
    let id: [u8; UDP_ID_LEN] = rand::random();
    Self { id, checksum: Self::compute_checksum(&id) }
  }

  #[inline]
  pub fn from(data: [u8; UDP_ID_LEN]) -> Self {
    Self { id: data, checksum: Self::compute_checksum(&data) }
  }

  #[inline]
  pub fn try_from(data_slice: &[u8]) -> Option<Self> {
    Some(Self { id: data_slice.get(0..UDP_ID_LEN)?.try_into().ok()?, checksum: data_slice.get(UDP_ID_LEN..UDP_ID_TOTAL_LEN)?.try_into().ok()? })
  }

  #[inline]
  pub fn validate(&self) -> bool {
    self.checksum == Self::compute_checksum(&self.id)
  }

  #[inline]
  pub fn as_u32(&self) -> u32 {
    u32::from_le_bytes(self.id)
  }

  #[inline]
  pub fn as_bytes(&self) -> [u8; UDP_ID_TOTAL_LEN] {
    let mut out = [0u8; UDP_ID_TOTAL_LEN];
    out[..UDP_ID_LEN].copy_from_slice(&self.id);
    out[UDP_ID_LEN..].copy_from_slice(&self.checksum);
    out
  }
}

impl Hash for UdpId {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) {
    state.write_u32(self.as_u32());
  }
}
