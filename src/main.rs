mod cli;
mod model;
mod replace;
mod rg;
mod ui;
mod util;

use std::process;

use rg::exec::run_ripgrep;
use ui::tui::Tui;

fn main() {
    let args = cli::parse_arguments();

    match run_ripgrep(&args.rg_args()) {
        Ok(rg_results) => {
            let result = Tui::new(&args, rg_results).start();

            // Restore terminal.
            if let Err(err) = Tui::restore_terminal() {
                eprintln!(
                    "Failed to restore terminal state, consider running the `reset` command. Error: {}",
                    err
                );
            }

            // Handle application result.
            match result {
                Ok(Some(mut replacement_criteria)) => {
                    // If we detected an encoding passed to `rg`, then use that.
                    if let Some(encoding) = args.rg_encoding() {
                        replacement_criteria.set_encoding(encoding);
                    }

                    match replace::perform_replacements(replacement_criteria) {
                        Ok(results) => eprintln!("{}", results),
                        Err(err) => {
                            eprintln!("An error occurred during replacement: {}", err);
                            process::exit(1);
                        }
                    }
                }
                Ok(None) => eprintln!("Cancelled"),
                Err(err) => {
                    eprintln!("An app error occurred: {}", err);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}
