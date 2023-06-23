//! _An interactive replacer for `ripgrep`._
//!
//! This is an interactive command line tool to make find and replacement easy.
//! It uses [`ripgrep`] to find, and then provides you with a simple interface to see
//! the replacements in real-time and conditionally replace matches.
//!
//! Some features:
//!
//! * ⚡ Super fast search results
//! * ✨ Interactive interface for selecting which matches should be replaced or not
//! * 🕶️ Live preview of the replacements
//! * 🧠 Replace using capturing groups (e.g., when using `/foo (\w+)/` replace with `bar $1`)
//! * 🦀 and more!
//!
//! Supported file encodings:
//!
//! * ASCII
//! * UTF8
//! * UTF16BE
//! * UTF16LE
//!
//! Other encodings are possibly supported but untested at the moment.
//! See [this issue](https://github.com/acheronfail/repgrep/issues/12) for more information.
//!
//! # Usage
//!
//! After installing, just use `rgr` (think: `rg` + `replace`).
//!
//! The arguments are:
//!
//! ```bash
//! rgr <rg arguments> # See `rgr --help` for more details
//! ```
//!
//! Here's an example where we ran the command:
//!
//! ```bash
//! rgr -C5 dreamcast
//! ```
//!
//! And have entered the replacement `flycast`:
//!
//! ![demo using rgr](./doc/demo.png)
//!
//! # Installation
//!
//! First and foremost, make sure you've installed `ripgrep` (AKA: `rg`).
//! To do so see the [`ripgrep` installation instructions].
//!
//! ### Precompiled binaries
//!
//! See the [releases] page for pre-compiled binaries.
//!
//! ### Via Cargo
//!
//! **NOTE**: The minimum Rust version required is `1.46.0`.
//!
//! ```bash
//! cargo install repgrep
//! ```
//!
//! ### Via Pacman (Arch Linux)
//!
//! Maintained by [orhun](https://github.com/orhun).
//!
//! [`repgrep`](https://archlinux.org/packages/extra/x86_64/repgrep/) can be installed
//! from the official repositories using [Pacman](https://wiki.archlinux.org/title/Pacman).
//!
//! ```bash
//! pacman -S repgrep
//! ```
//!
//! ### From Source (via Cargo)
//!
//! **NOTE**: The minimum Rust version required is `1.64.0`.
//!
//! ```bash
//! git clone https://github.com/acheronfail/repgrep/
//! cd repgrep
//! cargo install --path .
//! ```
//!
//! [`ripgrep`]: https://github.com/BurntSushi/ripgrep
//! [releases]: https://github.com/acheronfail/repgrep/releases
//! [`ripgrep` installation instructions]: https://github.com/BurntSushi/ripgrep/#installation

mod cli;
mod encoding;
mod model;
mod replace;
mod rg;
mod ui;
mod util;

use std::fs::File;
use std::{env, process};

use anyhow::Result;
use clap::crate_name;
use flexi_logger::{opt_format, FileSpec, Logger};
use rg::exec::run_ripgrep;
use ui::tui::Tui;

use crate::rg::read::read_messages;

fn init_logging() -> Result<::std::path::PathBuf> {
    let log_dir = env::temp_dir().join(format!(".{}", crate_name!()));
    let log_spec = if cfg!(debug_assertions) {
        FileSpec::default()
            .directory(env::current_dir().unwrap())
            .basename("rgr")
            .use_timestamp(false)
    } else {
        FileSpec::default().directory(&log_dir)
    };
    Logger::try_with_env()
        .expect("Please pass a valid RUST_LOG string, see: https://docs.rs/flexi_logger/latest/flexi_logger/struct.LogSpecification.html")
        .log_to_file(log_spec)
        .format(opt_format)
        .start()?;

    log::trace!("--- LOGGER INITIALISED ---");

    Ok(log_dir)
}

fn main() {
    let log_dir = match init_logging() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to initialise logger: {}", e);
            process::exit(1);
        }
    };

    macro_rules! exit_with_error {
        ($( $eprintln_arg:expr ),*) => {
            log::error!($( $eprintln_arg ),*);
            eprintln!($( $eprintln_arg ),*);
            if log::log_enabled!(log::Level::Error) {
                eprintln!("Logs available at: {}", log_dir.display());
            }
            process::exit(1);
        };
    }

    let args = match cli::parse_arguments() {
        Ok(args) => args,
        Err(e) => {
            cli::print_help();
            exit_with_error!("\nFailed to parse arguments, error: {}", e);
        }
    };

    macro_rules! run_ripgrep {
        () => {{
            let display_args = args.rg_args().into_iter().collect::<Vec<_>>();
            log::debug!("User args for rg: {:?}", display_args);
            run_ripgrep(args.rg_args())
        }};
    }

    let rg_json = match env::var(cli::ENV_JSON_FILE) {
        Ok(path) => {
            log::debug!(
                "Found {}={}, reading messages from file",
                cli::ENV_JSON_FILE,
                &path
            );
            match File::open(path) {
                Ok(json_file) => read_messages(json_file),
                Err(e) => {
                    log::warn!("Failed to open file: {}", e);
                    log::warn!("Falling back to running rg");
                    run_ripgrep!()
                }
            }
        }
        Err(_) => run_ripgrep!(),
    };

    match rg_json {
        Ok(rg_messages) => {
            let rg_cmdline: String = args
                .rg_args()
                .map(|s| s.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" ");

            let patterns = args.rg_patterns();
            let result = Tui::new().and_then(|tui| tui.start(rg_cmdline, rg_messages, patterns));

            // Restore terminal.
            if let Err(err) = Tui::restore_terminal() {
                log::warn!("Failed to restore terminal state: {}", err);
                eprintln!(
                    "Failed to restore terminal state, consider running the `reset` command. Error: {}",
                    err
                );
            }

            // Handle application result.
            match result {
                Ok(Some(mut replacement_criteria)) => {
                    // If we detected an encoding passed to `rg`, then use that.
                    if let Some(encoding) = args.encoding {
                        replacement_criteria.set_encoding(encoding);
                    }

                    match replace::perform_replacements(replacement_criteria) {
                        Ok(_) => {}
                        Err(err) => {
                            exit_with_error!("An error occurred during replacement: {}", err);
                        }
                    }
                }
                Ok(None) => eprintln!("Cancelled"),
                Err(err) => {
                    exit_with_error!("An app error occurred: {}", err);
                }
            }
        }
        Err(e) => {
            exit_with_error!("{}", e);
        }
    }
}
