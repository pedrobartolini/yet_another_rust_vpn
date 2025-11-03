use std::hash::Hash;
use std::hash::Hasher;

pub const UDP_ID_LEN: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UdpId([u8; UDP_ID_LEN]);

impl UdpId {
  #[inline]
  pub fn generate() -> Self {
    Self(rand::random())
  }

  #[inline]
  pub fn from(data: [u8; UDP_ID_LEN]) -> Self {
    Self(data)
  }

  #[inline]
  pub fn try_from(data_slice: &[u8]) -> Option<Self> {
    Some(Self(data_slice.get(0..UDP_ID_LEN)?.try_into().ok()?))
  }

  #[inline]
  pub fn as_u32(&self) -> u32 {
    u32::from_le_bytes(self.0)
  }
}

// --- Custom Hasher for integer keys ---
impl Hash for UdpId {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) {
    // hash directly as u32 — no need for per-byte iteration
    state.write_u32(self.as_u32());
  }
}
