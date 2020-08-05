use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fmt::Display;
use std::io::{ErrorKind, Read};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Error, Result};

use crate::rg::de::RgMessage;

fn rg_run_error(msg: impl Display) -> Error {
    anyhow!("An error occurred when running `rg`:\n\n{}", msg)
}

pub fn run_ripgrep<I, S>(args: I) -> Result<VecDeque<RgMessage>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = match Command::new("rg")
        // We use the JSON output
        .arg("--json")
        // We don't (yet?) support reading `rg`'s config files
        .arg("--no-config")
        .args(args)
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
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

    // Read messages from child process.
    let rg_messages = super::read::read_messages(child.stdout.as_mut().unwrap())?;

    // Wait for ripgrep to finish before returning.
    match child.wait() {
        Ok(exit_status) if exit_status.success() => Ok(rg_messages),
        Ok(_) => {
            let mut rg_stderr = String::new();
            Err(
                match child
                    .stderr
                    .as_mut()
                    .unwrap()
                    .read_to_string(&mut rg_stderr)
                {
                    Ok(_) => {
                        if rg_stderr.is_empty() {
                            anyhow!("No matches found")
                        } else {
                            rg_run_error(rg_stderr)
                        }
                    }
                    Err(e) => anyhow!("failed to read rg's stderr: {}", e),
                },
            )
        }
        Err(e) => Err(anyhow!("failed to wait for rg to end: {}", e)),
    }
}
