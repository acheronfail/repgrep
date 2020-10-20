mod cli;
mod encoding;
mod model;
mod replace;
mod rg;
mod ui;
mod util;

use std::env;
use std::process;

use anyhow::Result;
use clap::crate_name;
use flexi_logger::{opt_format, Logger};
use rg::exec::run_ripgrep;
use ui::tui::Tui;

fn init_logging() -> Result<::std::path::PathBuf> {
    let log_dir = env::temp_dir().join(format!(".{}", crate_name!()));
    Logger::with_env()
        .log_to_file()
        .directory(&log_dir)
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
            eprintln!("Logs available at: {}", log_dir.display());
            process::exit(1);
        };
    };

    let args = match cli::parse_arguments() {
        Ok(args) => args,
        Err(e) => {
            cli::print_help();
            exit_with_error!("\nFailed to parse arguments, error: {}", e);
        }
    };

    log::debug!(
        "User args for rg: {:?}",
        args.rg_args().into_iter().collect::<Vec<_>>()
    );
    match run_ripgrep(args.rg_args()) {
        Ok(rg_messages) => {
            let rg_cmdline: String = args
                .rg_args()
                .map(|s| s.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" ");

            let result = Tui::new(rg_cmdline, rg_messages).start();

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
