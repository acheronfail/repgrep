use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fmt::Display;
use std::io::{self, BufRead, BufReader, ErrorKind, Read, Write};
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

    let mut rg_messages: VecDeque<RgMessage> = VecDeque::new();
    let rg_stdout = BufReader::new(child.stdout.as_mut().unwrap());
    for (i, line) in rg_stdout.lines().enumerate() {
        // For large result lists show some progress in the terminal.
        if i > 0 && i % 1000 == 0 {
            let _ = io::stdout().write_all(format!("\rMatches found: ~{}", i).as_bytes());
            let _ = io::stdout().flush();
        }

        rg_messages.push_back(
            serde_json::from_str(&line?)
                .map_err(|e| anyhow!("Failed to read JSON output from `rg`: {}", e))?,
        );
    }

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
