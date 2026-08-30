use std::collections::{HashMap, HashSet};
use std::ops::Range;

use anyhow::{Result, anyhow, bail};
use grep_matcher::{Captures, Matcher};
use grep_pcre2::{RegexMatcher as Pcre2Matcher, RegexMatcherBuilder as Pcre2MatcherBuilder};
use grep_regex::{RegexMatcher as DefaultMatcher, RegexMatcherBuilder as DefaultMatcherBuilder};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RegexEngine {
    #[default]
    Default,
    Pcre2,
    Auto,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CaseMode {
    #[default]
    Sensitive,
    Insensitive,
    Smart,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegexConfig {
    pub engine: RegexEngine,
    pub case: CaseMode,
    pub multiline: bool,
    pub multiline_dotall: bool,
    pub crlf: bool,
    pub word: bool,
    pub whole_line: bool,
    pub unicode: bool,
}

impl Default for RegexConfig {
    fn default() -> RegexConfig {
        RegexConfig {
            engine: RegexEngine::Default,
            case: CaseMode::Sensitive,
            multiline: false,
            multiline_dotall: false,
            crlf: false,
            word: false,
            whole_line: false,
            unicode: true,
        }
    }
}

#[derive(Clone, Debug)]
pub enum CaptureMatcher {
    Default(DefaultMatcher),
    Pcre2(Pcre2Matcher),
}

impl CaptureMatcher {
    pub fn new(pattern: &str, config: &RegexConfig, fixed_strings: bool) -> Result<CaptureMatcher> {
        match config.engine {
            RegexEngine::Default => Self::new_default(pattern, config, fixed_strings),
            RegexEngine::Pcre2 => Self::new_pcre2(pattern, config, fixed_strings),
            RegexEngine::Auto => Self::new_default(pattern, config, fixed_strings)
                .or_else(|_| Self::new_pcre2(pattern, config, fixed_strings)),
        }
    }

    fn new_default(
        pattern: &str,
        config: &RegexConfig,
        fixed_strings: bool,
    ) -> Result<CaptureMatcher> {
        let mut builder = DefaultMatcherBuilder::new();
        builder
            .case_insensitive(config.case == CaseMode::Insensitive)
            .case_smart(config.case == CaseMode::Smart)
            // ripgrep always gives ^ and $ their line-oriented meanings.
            .multi_line(true)
            .dot_matches_new_line(config.multiline_dotall)
            .unicode(config.unicode)
            .crlf(config.crlf)
            .word(config.word)
            .whole_line(config.whole_line)
            .fixed_strings(fixed_strings);
        if config.multiline {
            builder.line_terminator(None);
        } else if !config.crlf {
            // In non-multiline mode, ripgrep rejects matches containing a line terminator.
            // `crlf(true)` has already installed the corresponding two-byte terminator.
            builder.line_terminator(Some(b'\n'));
        }
        builder
            .build(pattern)
            .map(CaptureMatcher::Default)
            .map_err(|error| anyhow!(error))
    }

    fn new_pcre2(
        pattern: &str,
        config: &RegexConfig,
        fixed_strings: bool,
    ) -> Result<CaptureMatcher> {
        let mut builder = Pcre2MatcherBuilder::new();
        builder
            .caseless(config.case == CaseMode::Insensitive)
            .case_smart(config.case == CaseMode::Smart)
            .multi_line(true)
            .dotall(config.multiline_dotall)
            .crlf(config.crlf)
            .word(config.word)
            .whole_line(config.whole_line)
            .fixed_strings(fixed_strings)
            .utf(config.unicode)
            .ucp(config.unicode);
        builder
            .build(pattern)
            .map(CaptureMatcher::Pcre2)
            .map_err(|error| anyhow!(error))
    }

    pub fn capture_count(&self) -> usize {
        match self {
            CaptureMatcher::Default(matcher) => matcher.capture_count(),
            CaptureMatcher::Pcre2(matcher) => matcher.capture_count(),
        }
    }

    pub fn replacements_for(
        &self,
        haystack: &[u8],
        ranges: impl IntoIterator<Item = Range<usize>>,
        replacement: &[u8],
    ) -> Result<HashMap<(usize, usize), Vec<u8>>> {
        let expected = ranges
            .into_iter()
            .map(|range| (range.start, range.end))
            .collect::<HashSet<_>>();
        if expected.is_empty() {
            return Ok(HashMap::new());
        }

        let replacements = match self {
            CaptureMatcher::Default(matcher) => {
                collect_replacements(matcher, haystack, &expected, replacement)?
            }
            CaptureMatcher::Pcre2(matcher) => {
                collect_replacements(matcher, haystack, &expected, replacement)?
            }
        };

        if replacements.len() != expected.len() {
            let resolved = replacements.keys().copied().collect::<HashSet<_>>();
            let missing = expected
                .difference(&resolved)
                .map(|(start, end)| format!("{start}..{end}"))
                .collect::<Vec<_>>()
                .join(", ");
            bail!("failed to resolve capture groups for match ranges: {missing}")
        }
        Ok(replacements)
    }

    pub fn replacement_for(
        &self,
        haystack: &[u8],
        range: Range<usize>,
        replacement: &[u8],
    ) -> Result<Vec<u8>> {
        self.replacements_for(haystack, [range.clone()], replacement)?
            .remove(&(range.start, range.end))
            .ok_or_else(|| anyhow!("failed to resolve capture groups for match range"))
    }
}

pub fn capture_pattern(
    patterns: &[String],
    config: &RegexConfig,
    fixed_strings: bool,
) -> Result<Option<CaptureMatcher>> {
    if fixed_strings {
        return Ok(None);
    }

    let mut matchers = patterns
        .iter()
        .map(|pattern| CaptureMatcher::new(pattern, config, false))
        .collect::<Result<Vec<_>>>()?;

    if matchers.len() == 1 {
        return Ok(matchers.pop());
    }

    if matchers.iter().any(|matcher| matcher.capture_count() > 1) {
        bail!(
            "Either pass a single pattern with capturing groups, or many patterns without capturing groups.\n\nPatterns:\n\n{}",
            patterns
                .iter()
                .map(|pattern| format!("  - {pattern}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    Ok(None)
}

fn collect_replacements<M: Matcher>(
    matcher: &M,
    haystack: &[u8],
    expected: &HashSet<(usize, usize)>,
    replacement: &[u8],
) -> Result<HashMap<(usize, usize), Vec<u8>>> {
    let mut captures = matcher.new_captures().map_err(|error| anyhow!("{error}"))?;
    let mut replacements = HashMap::new();
    matcher
        .captures_iter(haystack, &mut captures, |captures| {
            let Some(matched) = captures.get(0) else {
                return true;
            };
            let range = (matched.start(), matched.end());
            if expected.contains(&range) {
                let mut expanded = vec![];
                captures.interpolate(
                    |name| matcher.capture_index(name),
                    haystack,
                    replacement,
                    &mut expanded,
                );
                replacements.insert(range, expanded);
            }
            replacements.len() != expected.len()
        })
        .map_err(|error| anyhow!("{error}"))?;
    Ok(replacements)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn pcre2(pattern: &str) -> CaptureMatcher {
        CaptureMatcher::new(
            pattern,
            &RegexConfig {
                engine: RegexEngine::Pcre2,
                ..RegexConfig::default()
            },
            false,
        )
        .unwrap()
    }

    #[test]
    fn default_and_pcre2_expand_numbered_and_named_captures() {
        for engine in [RegexEngine::Default, RegexEngine::Pcre2] {
            let matcher = CaptureMatcher::new(
                r"(?P<first>foo) (?P<second>bar)",
                &RegexConfig {
                    engine,
                    ..RegexConfig::default()
                },
                false,
            )
            .unwrap();
            assert_eq!(
                matcher
                    .replacement_for(b"foo bar", 0..7, b"$second-$1-${first}-$$")
                    .unwrap(),
                b"bar-foo-foo-$"
            );
        }
    }

    #[test]
    fn pcre2_supports_backreferences_lookbehind_and_reset_start() {
        assert_eq!(
            pcre2(r"(\w+) \1")
                .replacement_for(b"word word", 0..9, b"$1")
                .unwrap(),
            b"word"
        );
        assert_eq!(
            pcre2(r"(?<=(ba))(r)")
                .replacement_for(b"bar", 2..3, b"$1-$2")
                .unwrap(),
            b"ba-r"
        );
        assert_eq!(
            pcre2(r"(ba)\K(r)")
                .replacement_for(b"bar", 2..3, b"$1-$2")
                .unwrap(),
            b"ba-r"
        );
    }

    #[test]
    fn auto_uses_default_then_falls_back_to_pcre2() {
        let config = RegexConfig {
            engine: RegexEngine::Auto,
            ..RegexConfig::default()
        };
        assert!(matches!(
            CaptureMatcher::new(r"(foo)", &config, false).unwrap(),
            CaptureMatcher::Default(_)
        ));
        assert!(matches!(
            CaptureMatcher::new(r"(\w+) \1", &config, false).unwrap(),
            CaptureMatcher::Pcre2(_)
        ));
    }

    #[test]
    fn honors_matching_options() {
        let config = RegexConfig {
            engine: RegexEngine::Pcre2,
            case: CaseMode::Insensitive,
            multiline: true,
            multiline_dotall: true,
            ..RegexConfig::default()
        };
        let matcher = CaptureMatcher::new(r"(foo).*(bar)", &config, false).unwrap();
        assert_eq!(
            matcher
                .replacement_for(b"FOO\nmiddle\nBAR", 0..14, b"$2-$1")
                .unwrap(),
            b"BAR-FOO"
        );
    }

    #[test]
    fn reports_missing_ripgrep_match_ranges() {
        let error = pcre2(r"(foo)")
            .replacement_for(b"foo", 1..3, b"$1")
            .unwrap_err();
        assert!(error.to_string().contains("1..3"));
    }

    #[test]
    fn prepares_capture_patterns_for_replacement() {
        let config = RegexConfig::default();
        assert!(capture_pattern(&[], &config, false).unwrap().is_none());
        assert!(
            capture_pattern(&["foo".into()], &config, false)
                .unwrap()
                .is_some()
        );
        assert!(
            capture_pattern(&["(foo)".into()], &config, true)
                .unwrap()
                .is_none()
        );
        assert!(
            capture_pattern(&["foo".into(), "bar".into()], &config, false)
                .unwrap()
                .is_none()
        );

        let error = capture_pattern(&["(foo)".into(), "bar".into()], &config, false).unwrap_err();
        assert!(error.to_string().contains("single pattern"));
    }
}
