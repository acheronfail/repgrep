use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fmt::Display;
use std::io::ErrorKind;
use std::process::Command;

use anyhow::{anyhow, Error, Result};

use crate::rg::de::RgMessage;

fn vec_to_string(v: Vec<u8>) -> String {
    String::from_utf8(v).unwrap().trim().to_string()
}

fn rg_run_error(msg: impl Display) -> Error {
    anyhow!("An error occurred when running `rg`:\n\n{}", msg)
}

pub fn run_ripgrep<I, S>(args: I) -> Result<VecDeque<RgMessage>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = match Command::new("rg")
        // We use the JSON output
        .arg("--json")
        // We don't (yet?) support reading `rg`'s config files
        .arg("--no-config")
        .args(args)
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            if let ErrorKind::NotFound = e.kind() {
                return Err(anyhow!(
                    "Failed to find `rg`! Please make sure it's installed and available in PATH."
                ));
            } else {
                return Err(rg_run_error(e));
            }
        }
    };

    if !output.status.success() {
        if output.stderr.is_empty() {
            return Err(anyhow!("No matches found"));
        } else {
            return Err(rg_run_error(vec_to_string(output.stderr)));
        }
    }

    let mut rg_messages: VecDeque<RgMessage> = VecDeque::new();
    for line in vec_to_string(output.stdout).lines() {
        rg_messages.push_back(
            serde_json::from_str(line)
            .map_err(|e| anyhow!("Failed to read JSON output from `rg`: {}\nMost likely arguments that conflict with --json were passed to `rg`.", e))?
        );
    }

    Ok(rg_messages)
}
