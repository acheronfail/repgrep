/// Note that while we duplicate a log of ripgrep's command line options here, we don't appear to
/// use many of them at all. This is because the Clap arguments defined here act as a whitelist of
/// supported ripgrep options: if this fails to pass, then the user has passed some options which we
/// don't yet support.
///
/// We do use some of this information, for instance the `encoding` is sniffed from the argument
/// parsing we do here.
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use clap::AppSettings::ColoredHelp;
use clap::Clap;
use clap::{crate_authors, crate_version};

// TODO: options to support in the future
// -P/--pcre2
// -F/--fixed-strings
// -U/--multiline
// --multiline-dotall
// -f/--file

/// See `rg --help` for more detailed information on each of the flags passed.
#[derive(Clap, Debug)]
#[clap(
  version = crate_version!(),
  author = crate_authors!(),
  setting = ColoredHelp,
)]
pub struct Args {
    /// The pattern to search. Required unless patterns are passed via -e/--regexp.
    #[clap(name = "PATTERN")]
    pub pattern: Option<String>,
    /// The paths in which to search.
    #[clap(name = "PATH", parse(from_os_str))]
    pub paths: Vec<PathBuf>,
    /// Used to provide multiple patterns.
    #[clap(short = "e", long = "regexp", multiple = true, number_of_values = 1)]
    pub patterns: Vec<String>,

    /// How many lines of context should be shown after each match.
    #[clap(short = "A", long = "after-context")]
    pub after_context: Option<usize>,
    /// How many lines of context should be shown before each match.
    #[clap(short = "B", long = "before-context")]
    pub before_context: Option<usize>,
    /// How many lines of context should be shown before and after each match.
    #[clap(short = "C", long = "context")]
    pub context: Option<usize>,
    /// Treat CRLF ('\r\n') as a line terminator.
    #[clap(long = "crlf")]
    pub crlf: bool,
    /// Provide the encoding to use when searching files.
    #[clap(short = "E", long = "encoding")]
    pub encoding: Option<String>,
    /// Follow symlinks.
    #[clap(short = "L", long = "follow")]
    pub follow_symlinks: bool,
    /// Ignore case when searching.
    #[clap(short = "i", long = "ignore-case")]
    pub ignore_case: bool,
    /// Invert the matches on each line.
    #[clap(short = "v", long = "invert-match")]
    pub invert_match: bool,
    /// Print both matching and non-matching lines.
    #[clap(long = "passthru")]
    pub passthru: bool,
    /// Use smart case matching.
    #[clap(short = "S", long = "smart-case")]
    pub smart_case: bool,
    /// Use case sensitive matching.
    #[clap(short = "s", long = "case-sensitive")]
    pub case_sensitive: bool,
    /// Sort the results (ascending).
    #[clap(long = "sort")]
    pub sort: Option<String>,
    /// Sort the results (descending).
    #[clap(long = "sortr")]
    pub sortr: Option<String>,
    /// How many threads to use.
    #[clap(short = "j", long = "threads")]
    pub threads: Option<usize>,
    /// Trim leading/trailing whitespace.
    #[clap(long = "trim")]
    pub trim: bool,
    /// Search only a specific type of file.
    #[clap(short = "t", long = "type", multiple = true, number_of_values = 1)]
    pub r#type: Vec<String>,
    /// Inverse of --type.
    #[clap(short = "T", long = "type-not", multiple = true, number_of_values = 1)]
    pub type_not: Vec<String>,
    /// Set the "unrestricted" searching options for ripgrep.
    /// Note that this is currently limited to only two occurrences `-uu` since
    /// binary searching is not supported in repgrep.
    #[clap(short = "u", long = "unrestricted", parse(from_occurrences))]
    pub unrestricted: usize,
    /// When matching, use a word boundary search.
    #[clap(short = "w", long = "word-regexp")]
    pub word_regexp: bool,

    /// A list of globs to match files.
    #[clap(short = "g", long = "glob", multiple = true, number_of_values = 1)]
    pub glob: Vec<String>,
    /// A list of case insensitive globs to match files.
    #[clap(long = "iglob", multiple = true, number_of_values = 1)]
    pub iglob: Vec<String>,
    /// Search hidden files.
    #[clap(long = "hidden")]
    pub hidden: bool,
    /// Use the given ignore file when searching.
    #[clap(long = "ignore-file")]
    pub ignore_file: Option<PathBuf>,
    /// When given an --ignore-file, read its rules case insensitively.
    #[clap(long = "ignore-file-case-insensitive")]
    pub ignore_file_case_insensitive: bool,
    /// Don't traverse filesystems for each path specified.
    #[clap(long = "one-file-system")]
    pub one_file_system: bool,
}

impl Args {
    /// Provides the command line arguments to pass down to ripgrep.
    /// At the moment this just proxies down _all_ command line arguments (excluding the program name)
    /// directly to ripgrep. We assume that the arguments contain a supported set of flags and options
    /// since we'll have used Clap to parse this struct and validate our program's arguments.
    pub fn rg_args(&self) -> impl Iterator<Item = OsString> {
        // Skip the first argument, which _should_ be the binary name.
        env::args_os().skip(1)
    }

    /// Returns the patterns used by `rg` in the search.
    #[allow(unused)]
    pub fn rg_patterns(&self) -> Vec<&str> {
        if let Some(pattern) = &self.pattern {
            vec![pattern]
        } else {
            self.patterns.iter().map(|p| p.as_ref()).collect()
        }
    }
}
