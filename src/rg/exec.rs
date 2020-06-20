use std::collections::VecDeque;
use std::process::Command;

use anyhow::{anyhow, Result};

use crate::cli;
use crate::rg::de::RgMessageType;

pub fn run_ripgrep(args: &cli::Args) -> Result<VecDeque<RgMessageType>> {
  let to_string = |s| String::from_utf8(s).unwrap().trim().to_string();
  let output = Command::new("rg")
    .arg("--json")
    .args(&args.rg_args)
    .output()?;

  if !output.status.success() {
    return Err(anyhow!(
      "An error occurred running rg: {}",
      to_string(output.stderr)
    ));
  }

  Ok(
    to_string(output.stdout)
      .lines()
      .map(|line| serde_json::from_str(line).unwrap())
      .collect(),
  )
}
