/// Note that while we duplicate a log of ripgrep's command line options here, we don't appear to
/// use many of them at all. This is because the Parser arguments defined here act as a whitelist of
/// supported ripgrep options: if this fails to pass, then the user has passed some options which we
/// don't yet support.
///
/// We do use some of this information, for instance the `encoding` is sniffed from the argument
/// parsing we do here.
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use clap::{crate_authors, crate_version};
use clap::{ArgAction, Parser};

// TODO: options to support in the future
// -P/--pcre2
// -F/--fixed-strings
// -f/--file

/// See `rg --help` for more detailed information on each of the flags passed.
///
/// Providing no arguments will make repgrep read JSON input from STDIN.
#[derive(Parser, Debug)]
#[clap(
  version = crate_version!(),
  author = crate_authors!(),
)]
pub struct Args {
    //
    // RIPGREP ARGUMENTS
    //

    // POSITIONAL
    /// The pattern to search. Required unless patterns are passed via -e/--regexp.
    #[clap(name = "PATTERN")]
    pub pattern: Option<String>,
    /// The paths in which to search.
    #[clap(name = "PATH")]
    pub paths: Vec<PathBuf>,
    /// Used to provide multiple patterns.
    #[clap(
        short = 'e',
        long = "regexp",
        num_args = 1..,
        number_of_values = 1
    )]
    pub patterns: Vec<String>,

    // FLAGS
    /// How many lines of context should be shown after each match.
    #[clap(short = 'A', long = "after-context")]
    pub after_context: Option<usize>,
    /// How many lines of context should be shown before each match.
    #[clap(short = 'B', long = "before-context")]
    pub before_context: Option<usize>,
    /// How many lines of context should be shown before and after each match.
    #[clap(short = 'C', long = "context")]
    pub context: Option<usize>,
    /// Treat CRLF ('\r\n') as a line terminator.
    #[clap(long = "crlf")]
    pub crlf: bool,
    /// Provide the encoding to use when searching files.
    #[clap(short = 'E', long = "encoding")]
    pub encoding: Option<String>,
    /// Follow symlinks.
    #[clap(short = 'L', long = "follow")]
    pub follow_symlinks: bool,
    /// Ignore case when searching.
    #[clap(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,
    /// Invert the matches on each line.
    #[clap(short = 'v', long = "invert-match")]
    pub invert_match: bool,
    /// Print both matching and non-matching lines.
    #[clap(long = "passthru")]
    pub passthru: bool,
    /// Use smart case matching.
    #[clap(short = 'S', long = "smart-case")]
    pub smart_case: bool,
    /// Use case sensitive matching.
    #[clap(short = 's', long = "case-sensitive")]
    pub case_sensitive: bool,
    /// Sort the results (ascending).
    #[clap(long = "sort")]
    pub sort: Option<String>,
    /// Sort the results (descending).
    #[clap(long = "sortr")]
    pub sortr: Option<String>,
    /// How many threads to use.
    #[clap(short = 'j', long = "threads")]
    pub threads: Option<usize>,
    /// Trim leading/trailing whitespace.
    #[clap(long = "trim")]
    pub trim: bool,
    /// Search only a specific type of file.
    #[clap(
        short = 't',
        long = "type",
        num_args = 1..,
        number_of_values = 1
    )]
    pub r#type: Vec<String>,
    /// Inverse of --type.
    #[clap(
        short = 'T',
        long = "type-not",
        num_args = 1..,
        number_of_values = 1
    )]
    pub type_not: Vec<String>,
    /// Set the "unrestricted" searching options for ripgrep.
    /// Note that this is currently limited to only two occurrences `-uu` since
    /// binary searching is not supported in repgrep.
    #[clap(short = 'u', long = "unrestricted", action = ArgAction::Count)]
    pub unrestricted: u8,
    /// Allow matches to span multiple lines.
    #[clap(short = 'U', long = "multiline")]
    pub multiline: bool,
    /// Allow the "." character to span across lines with multiline searches.
    #[clap(long = "multiline-dotall")]
    pub multiline_dotall: bool,
    /// When matching, use a word boundary search.
    #[clap(short = 'w', long = "word-regexp")]
    pub word_regexp: bool,

    // FILES & IGNORES
    /// A list of globs to match files.
    #[clap(
        short = 'g',
        long = "glob",
        num_args = 1..,
        number_of_values = 1
    )]
    pub glob: Vec<String>,
    /// A list of case insensitive globs to match files.
    #[clap(long = "iglob", num_args = 1.., number_of_values = 1)]
    pub iglob: Vec<String>,
    /// Search hidden files.
    #[clap(short = '.', long = "hidden")]
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
    /// since we'll have used Parser to parse this struct and validate our program's arguments.
    pub fn rg_args(&self) -> impl Iterator<Item = OsString> {
        // Skip the first argument, which _should_ be the binary name.
        env::args_os().skip(1)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::{CommandFactory, Parser};

    use super::Args;

    #[test]
    fn verify_cli() {
        Args::command().debug_assert()
    }

    #[test]
    fn verify_pattern() {
        let args = Args::parse_from(&["rgr", "foobar"]);
        assert_eq!(args.pattern, Some(String::from("foobar")));
    }

    #[test]
    fn verify_paths() {
        let args = Args::parse_from(&["rgr", "foobar", "/tmp", "/dev"]);
        assert_eq!(
            args.paths,
            vec![PathBuf::from("/tmp"), PathBuf::from("/dev")]
        );
    }

    #[test]
    fn verify_patterns() {
        let args = Args::parse_from(&["rgr", "-e=foobar", "-e", "pattern"]);
        assert_eq!(
            args.patterns,
            vec![String::from("foobar"), String::from("pattern")]
        );
    }

    #[test]
    fn verify_after_context() {
        let args = Args::parse_from(&["rgr", ".", "-A5"]);
        assert_eq!(args.after_context, Some(5));
    }

    #[test]
    fn verify_before_context() {
        let args = Args::parse_from(&["rgr", ".", "-B=10"]);
        assert_eq!(args.before_context, Some(10));
    }

    #[test]
    fn verify_context() {
        let args = Args::parse_from(&["rgr", ".", "-C", "42"]);
        assert_eq!(args.context, Some(42));
    }

    #[test]
    fn verify_crlf() {
        let args = Args::parse_from(&["rgr", ".", "--crlf"]);
        assert_eq!(args.crlf, true);
    }

    #[test]
    fn verify_encoding() {
        let args = Args::parse_from(&["rgr", ".", "-Eutf-8"]);
        assert_eq!(args.encoding, Some(String::from("utf-8")));
        let args = Args::parse_from(&["rgr", ".", "--encoding", "utf-16"]);
        assert_eq!(args.encoding, Some(String::from("utf-16")));
    }

    #[test]
    fn verify_follow_symlinks() {
        let args = Args::parse_from(&["rgr", ".", "-L"]);
        assert_eq!(args.follow_symlinks, true);
        let args = Args::parse_from(&["rgr", ".", "--follow"]);
        assert_eq!(args.follow_symlinks, true);
    }

    #[test]
    fn verify_ignore_case() {
        let args = Args::parse_from(&["rgr", ".", "-i"]);
        assert_eq!(args.ignore_case, true);
        let args = Args::parse_from(&["rgr", ".", "--ignore-case"]);
        assert_eq!(args.ignore_case, true);
    }

    #[test]
    fn verify_invert_match() {
        let args = Args::parse_from(&["rgr", ".", "-v"]);
        assert_eq!(args.invert_match, true);
        let args = Args::parse_from(&["rgr", ".", "--invert-match"]);
        assert_eq!(args.invert_match, true);
    }

    #[test]
    fn verify_passthru() {
        let args = Args::parse_from(&["rgr", ".", "--passthru"]);
        assert_eq!(args.passthru, true);
    }

    #[test]
    fn verify_smart_case() {
        let args = Args::parse_from(&["rgr", ".", "-S"]);
        assert_eq!(args.smart_case, true);
        let args = Args::parse_from(&["rgr", ".", "--smart-case"]);
        assert_eq!(args.smart_case, true);
    }

    #[test]
    fn verify_case_sensitive() {
        let args = Args::parse_from(&["rgr", ".", "-s"]);
        assert_eq!(args.case_sensitive, true);
        let args = Args::parse_from(&["rgr", ".", "--case-sensitive"]);
        assert_eq!(args.case_sensitive, true);
    }

    #[test]
    fn verify_sort() {
        let args = Args::parse_from(&["rgr", ".", "--sort=path"]);
        assert_eq!(args.sort, Some(String::from("path")));
    }

    #[test]
    fn verify_sortr() {
        let args = Args::parse_from(&["rgr", ".", "--sortr=created"]);
        assert_eq!(args.sortr, Some(String::from("created")));
    }

    #[test]
    fn verify_threads() {
        let args = Args::parse_from(&["rgr", ".", "-j12"]);
        assert_eq!(args.threads, Some(12));
        let args = Args::parse_from(&["rgr", ".", "--threads=4"]);
        assert_eq!(args.threads, Some(4));
    }

    #[test]
    fn verify_trim() {
        let args = Args::parse_from(&["rgr", ".", "--trim"]);
        assert_eq!(args.trim, true);
    }

    #[test]
    fn verify_type() {
        let args = Args::parse_from(&["rgr", ".", "-tcss"]);
        assert_eq!(args.r#type, vec![String::from("css")]);
        let args = Args::parse_from(&["rgr", ".", "-tcss", "--type=html"]);
        assert_eq!(args.r#type, vec![String::from("css"), String::from("html")]);
    }

    #[test]
    fn verify_type_not() {
        let args = Args::parse_from(&["rgr", ".", "-Tcss"]);
        assert_eq!(args.type_not, vec![String::from("css")]);
        let args = Args::parse_from(&["rgr", ".", "-Tcss", "--type-not=html"]);
        assert_eq!(
            args.type_not,
            vec![String::from("css"), String::from("html")]
        );
    }

    #[test]
    fn verify_unrestricted() {
        let args = Args::parse_from(&["rgr", ".", "-u"]);
        assert_eq!(args.unrestricted, 1);
        let args = Args::parse_from(&["rgr", ".", "-uu"]);
        assert_eq!(args.unrestricted, 2);
        let args = Args::parse_from(&["rgr", ".", "--unrestricted"]);
        assert_eq!(args.unrestricted, 1);
    }

    #[test]
    fn verify_multiline() {
        let args = Args::parse_from(&["rgr", ".", "--multiline"]);
        assert_eq!(args.multiline, true);
    }

    #[test]
    fn verify_multiline_dotall() {
        let args = Args::parse_from(&["rgr", ".", "--multiline-dotall"]);
        assert_eq!(args.multiline_dotall, true);
    }

    #[test]
    fn verify_word_regexp() {
        let args = Args::parse_from(&["rgr", ".", "-w"]);
        assert_eq!(args.word_regexp, true);
        let args = Args::parse_from(&["rgr", ".", "--word-regexp"]);
        assert_eq!(args.word_regexp, true);
    }

    #[test]
    fn verify_glob() {
        let args = Args::parse_from(&["rgr", ".", "-g=asdf"]);
        assert_eq!(args.glob, vec![String::from("asdf")]);
        let args = Args::parse_from(&["rgr", ".", "-g=asdf", "--glob", "qwerty"]);
        assert_eq!(
            args.glob,
            vec![String::from("asdf"), String::from("qwerty")]
        );
    }

    #[test]
    fn verify_iglob() {
        let args = Args::parse_from(&["rgr", ".", "--iglob=zxcv"]);
        assert_eq!(args.iglob, vec![String::from("zxcv")]);
    }

    #[test]
    fn verify_hidden() {
        let args = Args::parse_from(&["rgr", ".", "-."]);
        assert_eq!(args.hidden, true);
        let args = Args::parse_from(&["rgr", ".", "--hidden"]);
        assert_eq!(args.hidden, true);
    }

    #[test]
    fn verify_ignore_file() {
        let args = Args::parse_from(&["rgr", ".", "--ignore-file=my/path/to/.gitignore"]);
        assert_eq!(
            args.ignore_file,
            Some(PathBuf::from("my/path/to/.gitignore"))
        );
    }

    #[test]
    fn verify_ignore_file_case_insensitive() {
        let args = Args::parse_from(&["rgr", ".", "--ignore-file-case-insensitive"]);
        assert_eq!(args.ignore_file_case_insensitive, true);
    }

    #[test]
    fn verify_one_file_system() {
        let args = Args::parse_from(&["rgr", ".", "--one-file-system"]);
        assert_eq!(args.one_file_system, true);
    }
}
