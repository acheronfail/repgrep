use clap::AppSettings::{AllowLeadingHyphen, ColoredHelp, TrailingVarArg};
use clap::Clap;
use clap::{crate_authors, crate_version};

// TODO: configure:
//  replace: confidence level of character encoding detection
//  replace: disable strict byte assertions

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
  pub rg_args: Vec<String>,

  /// If the encoding was passed to `rg` then this is the value of that flag.
  #[clap(skip)]
  rg_encoding: Option<String>,
}

impl Args {
  pub fn rg_encoding(&self) -> Option<&String> {
    self.rg_encoding.as_ref()
  }
}

const RG_ENCODING_FLAGS: [&str; 5] = [" -E ", " -E=", " -E", " --encoding ", " --encoding="];

pub fn parse_arguments() -> Args {
  let mut args = Args::parse();

  // Try to find the encoding passed to `rg`.
  let rg_args_as_string = args.rg_args.join(" ");
  args.rg_encoding = RG_ENCODING_FLAGS.iter().find_map(|flag| {
    if let Some(start_index) = &rg_args_as_string.find(flag) {
      Some(
        rg_args_as_string
          .chars()
          .skip(start_index + flag.len())
          .take_while(|c| *c != ' ')
          .collect::<String>(),
      )
    } else {
      None
    }
  });

  args
}

#[cfg(test)]
mod tests {
  #[test]
  fn it_finds_rg_encoding() {
    // TODO: one for each RG_ENCODING_FLAGS
  }
}
