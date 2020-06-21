mod cli;
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
            if let Err(err) = Tui::restore_terminal() {
                eprintln!(
                    "Failed to restore terminal state, consider running the `reset` command. Error: {}",
                    err
                );
            }

            if let Err(err) = result {
                eprintln!("An error occurred: {}", err);
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}
