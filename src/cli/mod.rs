use std::{fs, process};

use anyhow::{bail, Result};
use lexopt::Parser;

pub const ENV_JSON_FILE: &str = "RGR_JSON_FILE";

pub fn print_help() {
    eprintln!(
        r#"{crate_name} {crate_version}
{crate_authors}

{crate_name} ({crate_bin_name}) is an interactive replacer for ripgrep that makes it easy to find
and replace across files on the command line.

Project home page: {crate_homepage}

USAGE:
    {crate_bin_name} <rg-args>...
    rg --json ... | {crate_bin_name} <rg-args>...
"#,
        crate_name = env!("CARGO_PKG_NAME"),
        crate_bin_name = env!("CARGO_BIN_NAME"),
        crate_version = env!("CARGO_PKG_VERSION"),
        crate_homepage = env!("CARGO_PKG_HOMEPAGE"),
        crate_authors = env!("CARGO_PKG_AUTHORS")
            .split(':')
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

pub struct Args {
    /// All the regular expressions that were passed. We need these since we perform matching
    /// ourselves in certain situations when rendering the TUI.
    pub patterns: Vec<String>,
    /// Any encoding that was passed - we want to force the same encoding that ripgrep uses when
    /// we perform any replacements ourselves.
    pub encoding: Option<String>,
    /// Whether fixed strings was enabled - means we only need to substring search rather than
    /// regular expression searching.
    /// TODO: this is currently unused, we need to update `replace.rs` to use it
    pub fixed_strings: bool,
    /// All other args that were passed will be forwarded to ripgrep.
    pub other_args: Vec<String>,
}

impl Args {
    pub fn parse() -> Result<Args> {
        Args::parse_arguments(Parser::from_env())
    }

    // TODO: this implementation assumes UTF-8 (via `String`) for all arguments, but in reality it
    // should use `OsString` instead to remove the UTF-8 requirement.
    fn parse_arguments(mut parser: Parser) -> Result<Args> {
        use lexopt::prelude::*;

        // ripgrep's arguments that we want to know
        let mut pattern_positional: Option<String> = None;
        let mut patterns: Vec<String> = vec![];
        let mut encoding: Option<String> = None;
        let mut fixed_strings = false;
        let mut other_args: Vec<String> = vec![];

        // as per ripgrep's documentation:
        // > When -f/--file or -e/--regexp is used, then ripgrep treats all positional arguments as
        // > files or directories to search.
        let mut positional_disabled = false;

        while let Some(arg) = parser.next()? {
            match arg {
                // ripgrep: pattern related arguments
                Value(pattern) if pattern_positional.is_none() => {
                    pattern_positional = Some(pattern.string()?);
                }
                Short('e') | Long("regexp") => {
                    positional_disabled = true;
                    patterns.push(parser.value()?.string()?);
                }
                Short('f') | Long("file") => {
                    positional_disabled = true;
                    let path = parser.value()?;
                    if path == "-" {
                        bail!("reading stdin for --file arguments is not yet supported in rgr")
                    }

                    let text = fs::read_to_string(path)?;
                    for pattern in text.lines() {
                        patterns.push(pattern.into());
                    }
                }

                // ripgrep: flags
                Short('E') | Long("encoding") => {
                    encoding = Some(parser.value()?.string()?);
                }
                Short('F') | Long("fixed-strings") => {
                    fixed_strings = true;
                }
                Long("no-fixed-strings") => {
                    fixed_strings = false;
                }

                // capture help to display our help
                Long("help") => {
                    print_help();
                    process::exit(0);
                }

                // ripgrep: all other arguments and flags
                Short(ch) => other_args.push(format!("-{}", ch)),
                Long(name) => {
                    // at this point we don't know if the argument we're passing is a `--flag` or an
                    // `--option=something`. So, peek at the next argument (if any) and see if it
                    // starts with `-`.
                    let name = name.to_string();
                    let next_is_flag = parser
                        .try_raw_args()
                        .map(|raw_args| {
                            raw_args
                                .peek()
                                .and_then(|next| next.to_str())
                                // if there's no next value, this must be a flag
                                // if there is a next value, see if it looks like a flag
                                .map_or(true, |s| s.starts_with('-'))
                        })
                        // if `try_raw_args` failed, then we're passing something with an optional
                        // value, so that's not a flag
                        .unwrap_or(false);

                    if next_is_flag {
                        other_args.push(format!("--{}", name));
                    } else {
                        other_args.push(format!("--{}={}", name, parser.value()?.string()?));
                    }
                }
                Value(other) => other_args.push(other.string()?),
            }
        }

        if let Some(pattern) = pattern_positional {
            if positional_disabled {
                other_args.push(pattern);
            } else {
                patterns.push(pattern);
            }
        }

        Ok(Args {
            patterns,
            fixed_strings,
            encoding,
            other_args,
        })
    }

    pub fn rg_args(&self) -> Vec<String> {
        let mut args = self.other_args.clone();
        if self.fixed_strings {
            args.push("--fixed-strings".into());
        }
        if let Some(encoding) = &self.encoding {
            args.push(format!("--encoding={}", encoding));
        }
        for pattern in &self.patterns {
            args.push(format!("--regexp={}", pattern));
        }

        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temp_file;

    macro_rules! parse {
        [$($arg:expr$(,)?)*] => {
            Args::parse_arguments(Parser::from_iter(["rgr".to_string(), $($arg.into(),)*])).unwrap()
        };
    }

    #[test]
    fn empty() {
        let args = parse![];
        assert!(args.patterns.is_empty());
        assert!(!args.fixed_strings);
        assert!(args.other_args.is_empty());
        assert_eq!(args.encoding, None);
    }

    #[test]
    fn patterns() {
        // only positional
        let args = parse!["positional"];
        assert_eq!(args.patterns, ["positional"]);
        assert!(args.other_args.is_empty());

        // positional and --regexp
        let args = parse!["positional", "--regexp=e"];
        assert_eq!(args.patterns, ["e"]);
        assert_eq!(args.other_args, ["positional"]);

        // positional and multiple --regexp flags
        let args = parse![
            "-e",
            "e1",
            "positional",
            "--regexp=e2",
            "-e=e3",
            "another_positional"
        ];
        assert_eq!(args.patterns, ["e1", "e2", "e3"]);
        assert_eq!(args.other_args, ["another_positional", "positional"]);
    }

    #[test]
    fn pattern_files() {
        let p = temp_file!("foo\nbar");

        // just --file
        let args = parse![format!("--file={}", p.display())];
        assert_eq!(args.patterns, ["foo", "bar"]);
        assert!(args.other_args.is_empty());

        // with positional
        let args = parse![format!("--file={}", p.display()), "positional"];
        assert_eq!(args.patterns, ["foo", "bar"]);
        assert_eq!(args.other_args, ["positional"]);

        // with positional and --regexp
        let args = parse![
            "positional",
            "-e=baz",
            format!("--file={}", p.display()),
            "another_positional"
        ];
        assert_eq!(args.patterns, ["baz", "foo", "bar"]);
        assert_eq!(args.other_args, ["another_positional", "positional"]);
    }

    #[test]
    fn fixed_strings() {
        let args = parse!["-F"];
        assert!(args.fixed_strings);

        let args = parse!["--fixed-strings"];
        assert!(args.fixed_strings);

        let args = parse!["--fixed-strings", "--no-fixed-strings"];
        assert!(!args.fixed_strings);
    }

    #[test]
    fn encoding() {
        let args = parse![];
        assert_eq!(args.encoding, None);

        let args = parse!["--encoding=utf-16be"];
        assert_eq!(args.encoding.as_deref(), Some("utf-16be"));

        let args = parse!["--encoding", "utf-16le"];
        assert_eq!(args.encoding.as_deref(), Some("utf-16le"));

        let args = parse!["-E", "utf-8"];
        assert_eq!(args.encoding.as_deref(), Some("utf-8"));

        let args = parse!["-Eascii"];
        assert_eq!(args.encoding.as_deref(), Some("ascii"));
    }

    #[test]
    fn other_args() {
        let args = parse![
            "pos1",
            "pos2",
            "--bool",
            "--flag1=val1",
            "--flag2",
            "val2",
            "-a",
            "-1"
        ];
        assert_eq!(args.patterns, ["pos1"]);
        assert_eq!(
            args.other_args,
            ["pos2", "--bool", "--flag1=val1", "--flag2=val2", "-a", "-1"]
        );
        assert!(!args.fixed_strings);
        assert!(args.encoding.is_none());

        assert_eq!(
            args.rg_args(),
            [
                "pos2",
                "--bool",
                "--flag1=val1",
                "--flag2=val2",
                "-a",
                "-1",
                "--regexp=pos1"
            ]
        );
    }

    #[test]
    fn case1() {
        let args = parse!["--sort", "path", "--sort=modified", "foo"];
        assert_eq!(
            args.rg_args(),
            ["--sort=path", "--sort=modified", "--regexp=foo"]
        );
    }

    #[test]
    fn case2() {
        let args = parse!["--flag"];
        assert_eq!(args.rg_args(), ["--flag"]);

        let args = parse!["--flag", "val"];
        assert_eq!(args.rg_args(), ["--flag=val"]);

        let args = parse!["--flag=val"];
        assert_eq!(args.rg_args(), ["--flag=val"]);
    }
}
