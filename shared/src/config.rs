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

  pub fn new_from_embed() -> anyhow::Result<Self> {
    let server_addr = include_str!("../../env/server_addr").trim().to_string();
    let server_port = include_str!("../../env/server_port").trim().parse()?;
    Ok(Self { server_addr, server_port })
  }
}
