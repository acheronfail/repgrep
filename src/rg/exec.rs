use std::collections::VecDeque;
use std::process::Command;

use anyhow::{anyhow, Result};

use crate::rg::de::RgMessage;

pub fn run_ripgrep(args: &[String]) -> Result<VecDeque<RgMessage>> {
  if args.is_empty() {
    return Err(anyhow!(
      "No arguments provided. Please pass arguments that will be forwarded to rg.\nSee rgr --help."
    ));
  }

  let to_string = |s| String::from_utf8(s).unwrap().trim().to_string();
  let output = Command::new("rg")
    .arg("--json")
    .args(args)
    .output()
    .expect("failed to run `rg`! Please make sure it's installed and available in PATH");

  if !output.status.success() {
    let stderr = to_string(output.stderr);
    if stderr.is_empty() {
      return Err(anyhow!("No matches found"));
    } else {
      return Err(anyhow!("An error occurred running rg:\n\n{}", stderr));
    }
  }

  Ok(
    to_string(output.stdout)
      .lines()
      .map(|line| serde_json::from_str(line).expect("failed to deserialise `rg` JSON output!"))
      .collect(),
  )
}
