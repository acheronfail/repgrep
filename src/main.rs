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

    match run_ripgrep(&args) {
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
                Ok(Some(replacement_criteria)) => {
                    match replace::perform_replacements(replacement_criteria) {
                        Ok(results) => println!("{}", results),
                        Err(err) => {
                            eprintln!("An error occurred during replacement: {}", err);
                            process::exit(1);
                        }
                    }
                }
                Ok(None) => println!("Cancelled"),
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
