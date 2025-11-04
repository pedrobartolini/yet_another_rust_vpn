#[derive(serde::Deserialize)]
pub struct Config {
  pub server_addr: String,
  pub server_port: u16
}

impl Config {
  pub fn new() -> anyhow::Result<Self> {
    dotenv::dotenv()?;
    Ok(envy::from_env()?)
  }
}
