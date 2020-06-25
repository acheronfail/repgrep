use clap::AppSettings::{AllowLeadingHyphen, ColoredHelp, TrailingVarArg};
use clap::Clap;
use clap::{crate_authors, crate_version};

// TODO: configure:
//  replace: confidence level of character encoding detection
//  replace: disable strict byte assertions

const RG_ENCODING_FLAGS: [&str; 5] = [" -E ", " -E=", " -E", " --encoding ", " --encoding="];

#[derive(Clap, Debug)]
#[clap(
  version = crate_version!(),
  author = crate_authors!(),
  setting = ColoredHelp,
  // These help us pass all arguments through to `rg`.
  setting = TrailingVarArg,
  setting = AllowLeadingHyphen,
)]
pub struct Args {
    /// Arguments to pass to `rg`. Bear in mind that the `--json` flag is always passed down.
    /// See the `--json` section under `rg --help` for more details.
    rg_args: Vec<String>,
}

impl Args {
    /// Get the arguments to pass to `rg`.
    /// This method filters out some invalid arguments that conflict with `--json`.
    pub fn rg_args(&self) -> Vec<String> {
        self.rg_args
            .iter()
            .filter(|arg| match &arg[..] {
                "-h"
                | "--help"
                | "-r" // shorthand for --replace
                | "--replace"
                | "-o" // shorthand for --only-matching
                | "--only-matching"
                | "--heading"
                | "-M" // shorthand for --max-columns
                | "--max-columns"
                | "--no-json"
                | "--files"
                | "-l" // shorthand for --files-with-matches
                | "--files-with-matches"
                | "--files-without-match"
                | "-c" // shorthand for --count
                | "--count"
                | "--count-matches" => false,
                _ => true,
            })
            .cloned()
            .collect()
    }

    /// If the encoding was passed to `rg` then this is the value of that flag.
    pub fn rg_encoding(&self) -> Option<String> {
        let rg_args_as_string = self.rg_args.join(" ");
        RG_ENCODING_FLAGS.iter().find_map(|flag| {
            if let Some(start_index) = &rg_args_as_string.find(flag) {
                let encoding = rg_args_as_string
                    .chars()
                    .skip(start_index + flag.len())
                    .take_while(|c| *c != ' ')
                    .collect::<String>();

                match &encoding[..] {
                    "=" | "" => None,
                    _ => Some(encoding),
                }
            } else {
                None
            }
        })
    }
}

pub fn parse_arguments() -> Args {
    Args::parse()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::cli::*;

    fn new_args(rg_args: &[&str]) -> Args {
        Args {
            rg_args: rg_args.iter().map(|s| String::from(*s)).collect(),
        }
    }

    #[test]
    fn it_finds_rg_encoding() {
        RG_ENCODING_FLAGS.iter().for_each(|flag| {
            let flag_with_encoding = format!("{}encoding", flag);
            let expected = Some("encoding".to_owned());

            // At the start.
            let args = new_args(&[&flag_with_encoding, "pattern", "-A1"]);
            assert_eq!(args.rg_encoding(), expected);

            // In the middle.
            let args = new_args(&["pattern", &flag_with_encoding, "-A1"]);
            assert_eq!(args.rg_encoding(), expected);

            // At the end.
            let args = new_args(&["pattern", "-A1", &flag_with_encoding]);
            assert_eq!(args.rg_encoding(), expected);

            // Without.
            let args = new_args(&["pattern", "-A1"]);
            assert_eq!(args.rg_encoding(), None);

            // Without encoding value.
            let args = new_args(&["pattern", "-A1", flag]);
            assert_eq!(args.rg_encoding(), None);
        });
    }
}
