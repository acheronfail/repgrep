use clap::AppSettings::{AllowLeadingHyphen, ColoredHelp, TrailingVarArg};
use clap::Clap;
use clap::{crate_authors, crate_version};

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
}

pub fn parse_arguments() -> Args {
  Args::parse()
}
