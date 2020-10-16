# Changelog

## Unreleased

...

## 0.8.0

- fae1f09 (origin/feat/multline-match-support, feat/multline-match-support) fix: update pinned rust version
- fae1678 chore: upgrade dependencies
- fae1a13 fix: use Vec<Spans> rather than Vec<ListItem> since they have PartialEq
- fae1467 chore: add tests for new multiline behaviour
- fae1402 chore: fix tests
- fae1a67 refactor: swap VecDeque for a plain ol' Vec
- fae197b chore: remove unneeded trait ToListItem
- fae1437 chore: cargo fmt
- fae1613 feat: allow multiline matches to be placed on a single line
- fae1471 refactor: initial implementation of multiline match support
- fae1c49 refactor: build `ListItem`s from `Item`s for the UI
- fae1adf feat: allow matches to span multiple lines
- fae1c7e refactor: separate ListState from selected item and match

## 0.7.5

- fae1a3d fix(dev): fixup justfile bump command
- fae1011 fix: re-word keybinding menu and fix formatting

## 0.7.4

- fae162f fix(ui): allow scrolling the text in the help view
- fae141b fix(ui): maintain newlines in help menu

## 0.7.3

- fae1d59 fix(fmt): build.rs
- fae18c8 fix(ci): recognise tags without preceding "v"
- fae1350 dev: add a justfile for common commands
- fae1304 fix: issues with an old Cargo.lock and beta versions of clap

## v0.7.2

- fae10d8 fix: crate a post-action that publishes the crate after other actions

## v0.7.1

- fae10ec fix: add step to publish crate in CI
- fae11ed feat: style select mode more clearly
- fae140b fix: handle case when rg returns no matches
- fae16ee refactor: abstract reading rg messages into another module
- fae1b2a feat: replace non-printable whitespace with symbols
- fae1135 chore: update README.md with png rather than gif

## v0.7.0

- fae1bfd feat: show cursor when inputting replacement
- fae1c17 chore: upgrade to newly release tui-rs 0.10.0
- fae10d6 chore: write tests for decoding ArbitraryData into OsString
- fae1265 refactor: use a block when defining a base iterator
- fae15b7 fix: include new keybinding in help menu
- fae10cd chore: add tests for toggling items
- fae1159 chore: more tests around non UTF8 in matches view
- fae1e28 ui: make it easier to see that files are highlighted
- fae102e fix: total replacements were inaccurate with failed sub matches
- fae1cf5 chore: add more tests for Item
- fae18b1 chore: minor tweaks to logs during and after replacement
- fae101c docs: add linked issue with crossterm bug
- fae1aae ci: pin version 1.43.0 as new minimum
- fae1f4f chore: update base64 test with an additional non UTF8 byte
- fae1497 fix: ensure UTF8 replacement char doesn't break alignment
- fae1d93 chore: re-enable and fixup tests
- fae131e refactor: simplify and clean some things up
- fae10fe wip: implement selecting submatches
- fae1199 wip: use new Spans from tui-rs
- fae106d refactor: clean up perform_replacements()
- fae14e4 refactor: force ASCII encoder when detected rather than windows-1252
- fae186e fix: replace all matches in a file at the same time
- fae1d9d refactor: stream in ripgrep's results
- fae1fdd refactor: remove replacement results and just log output

## v0.6.0

- fae1f96 docs: update README and manpage documentation for encoding
- fae1bd2 chore: add tests for replacing with shorter text
- fae1ae6 refactor: minor change to drop file bytes sooner
- fae1729 fix: add workaround for UTF8 with BOM
- fae1c962 feat: properly support UTF16 encodings

## v0.5.0

- fae12cf refactor: create cli mod to split arg definition and validation
- fae19a0 refactor: major refactor to CLI parsing to sniff rg arguments
- fae1166 Revert "refactor: minor fixup for Windows systems"
- fae1af0 refactor: minor fixup for Windows systems
- fae1377 fix: filter out more invalid rg args and update manpage

## v0.4.10

- fae1a06 fix: improved error handling when running ripgrep
- fae1ab83 fix: continue on error rather than bailing out

## v0.4.6 - v0.4.9

- Various issues at manpage generation and testing out CI

## v0.4.5

- fae1510 ci: update github action build names
- fae1e3f chore: merge help.txt and manpage generation
- fae1014 ci: add x86_64-unknown-linux-gnu target to release actions

## v0.4.4

- fae144a chore: remove final new ci testing configuration
- fae1a66 chore: move ci2 dir over old ci
- fae10f9 ci: switch from TravisCI to GitHub Actions
- fae1293 fix: issues with initial Windows builds
- fae13a0 chore: rustfmt

## v0.4.3

- fae1c56 fix: return errors running rg without a panic

## v0.4.2

- fae1a7e fix: better error handling for when ripgrep is not installed
- fae166f doc: update README.md

## v0.4.1

- fae13be travis: fix issue deploying GitHub releases
- fae1a99 chore: write tests for detecting rg encoding
- fae1169 ui: add an extra newline before final ReplacementResult stats

## v0.4.0

- fae193a feat: read encoding passed to rg and use that as override
- fae1015 refactor: only pass rg args to run_ripgrep()

## v0.3.3

- fae103d fix: alignment issues rendering non UTF8 items as Text structs
- fae15c4 chore: handle non UTF8 paths when converting Item to Text
- fae12f5 chore: make demo.gif smaller and easier to see

## v0.3.2

- fae18c6 doc: update demo.gif
- fae1bb2 fix: incorrect number of reported matches in results list
- fae1ebe doc: swap mp4 for gif so it works in the preview
- fae14fc doc: add demo recording and add it to the README
- fae10ce fix: handle UTF-8 BOM if found
- fae1059 refactor: simplify and improve the replacement results

## v0.3.1

- fae1138 feat: show <empty> in main view when replacement string is empty
- fae178b chore: update README

## v0.3.0

- fae1f65 feat: support encodings other than UTF-8
- fae13d0 refactor: simplify temp_rg_msg helper fn
- fae1043 refactor: simplify test by sharing variables
- fae167a chore: add tests to ensure only Match items are replaced
- fae1fa3 travis: fix issue with multiple builds trying to publish to crates
- fae1d31 chore: sort dependencies alphabetically in Cargo.toml

## v0.2.0

- fae19b2 fix: issue when replacing multiple matches in a single file
- fae1292 feat: return and log a replacement result
- fae12f1 fix: return error when no arguments are provided

## v0.1.1

- fae16e8 fix: issue replacing matches with an offset > 0
- fae1995 chore: update comment
- fae1781 chore: tests for perform_replacements() and many test utilities
- fae1275 fix: use correct Apache-2.0 SPDX identifier
- fae1ce0 fix: only have 5 keywords in Cargo.toml
- fae1597 chore: add homepage and repository to Cargo.toml
- fae15f5 travis: add initial configuration

## v0.1.0

Initial release
