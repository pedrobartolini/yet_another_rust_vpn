use std::process::Command;

pub fn setup_route(interface_name: &str) -> anyhow::Result<()> {
  let interface_index = get_interface_index(interface_name)?;
  add_routes(interface_index)
}

fn get_interface_index(interface_name: &str) -> anyhow::Result<u32> {
  let output = Command::new("netsh").args(&["interface", "ipv4", "show", "interfaces"]).output()?;

  let output_str = String::from_utf8_lossy(&output.stdout);

  // Parse the output to find our interface
  for line in output_str.lines() {
    if line.contains(interface_name) {
      // Line format: "Idx     Met         MTU          State                Name"
      // Extract the index (first number)
      let parts: Vec<&str> = line.split_whitespace().collect();
      if let Some(idx_str) = parts.first() {
        if let Ok(idx) = idx_str.parse::<u32>() {
          return Ok(idx);
        }
      }
    }
  }

  Err(anyhow::anyhow!("Could not find interface index for {}", interface_name))
}

fn add_routes(interface_index: u32) -> anyhow::Result<()> {
  Command::new("route").args(&["add", "0.0.0.0", "mask", "128.0.0.0", "0.0.0.0", "if", &interface_index.to_string(), "metric", "10"]).output()?;
  Command::new("route").args(&["add", "128.0.0.0", "mask", "128.0.0.0", "0.0.0.0", "if", &interface_index.to_string(), "metric", "10"]).output()?;
  Ok(())
}
