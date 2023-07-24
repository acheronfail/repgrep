# 0.14.2

- fae129b fix some platforms inputting multiple characters
- fae1d19 fix incorrect fallback encoding in man page
- 32378e0 Merge pull request #90 from acheronfail/next
- 9d97af6 Merge pull request #88 from acheronfail/feat-movable-input-cursor

# 0.14.0 & 0.14.1

- fae18c7 a little bling for the README
- fae1a46 add documentation about capturing groups
- fae1d7c snapshot tests for item rendering
- fae1aed improve edge case of ascii encoding detection
- fae1591 unit tests for replacements with capturing groups
- fae1018 feat: add ability to user capturing groups in replacements
- fae1488 dev: update justfile with default rule
- fae1e2a update DEVELOPMENT_NOTES.md

# 0.13.0

- fae14d5 remove tag from just bump: we'll do it manually from now on
- fae1fac one more assertion
- fae1a88 remember cursor position when moving back from confirm
- fae1b735 add tests for movable cursor
- fae14b1 implement a movable cursor when inputting replacement text
- 6c69f89 Merge pull request #87 from acheronfail/ci/check-readme
- f2415cc Merge pull request #86 from orhun/docs/update_readme
- 5d287f0 Update main.rs
- fae14f9 install ripgrep in ci
- fae1ab5 update release profile
- fae1345 create new ci step to check readme generation
- fae1691e update justfile
- 2ae4989 docs: update README.md about official Arch Linux package

# 0.12.3 & 0.12.4

- fae1b46 update justfile
- 2d3cc3c Merge pull request #84 from acheronfail/fix-permissions
- fae15d3 fix: ensure replaced files have the same permissions/mode
- 3778c5b Merge pull request #81 from orhun/docs/update_readme
- da21428 Update README.md
- 3b6be87 Merge pull request #82 from acheronfail/wip/benchmark-json
- fae1e61 fix: why now... I really dislike CIs
- fae1a0b add: benchmarks for parsing json
- d1d1f3f docs: update README.md about installing on Arch Linux
- 9a6ea57 Merge pull request #79 from acheronfail/fix/incorrect-log-message

# 0.12.2

- 8cdce3d Merge pull request #78 from acheronfail/fix/atomic-replacements-across-filesystems
- fae175c fix: only mention log dir if logs are enabled

# 0.12.1

- fae1d38 fix: create temp files next to original files
- af37ebb Merge pull request #77 from acheronfail/feat/ability-to-invert-selection

# 0.12.0

Added a new feature: the ability to "invert" selections:

- A single item can be "inverted" (same as toggling it on/off).
- A file can be "inverted" (invert each match inside file)
- All matches can be "inverted" (every match is inverted)

This feature can be used with the `v` and `V` keys.
See the in-app help (`?`) for more.

- fae1e65 add: tests for new invert selection feature
- fae11be feat: ability to invert selection
- 2717297 Merge pull request #76 from acheronfail/feat/performance-improvements

# 0.11.0

Significant performance improvements, especially with large result sets from ripgrep.
Now, only the visible portion of the matches is rendered, rather than everything at once.

- fae17d0 0.11.0
- fae1c8d add: DEVELOPMENT_NOTES.md
- fae144b doc: add comment to PartialEq impl for Item
- fae12df upd: add cache for item line count; fix: some rendering issues with windowing
- fae1395f upd: only render visible lines
- fae1068 upd: decrease input flush threshold
- fae12c4a perf: build printable string in a single iteration rather than multiple
- fae1232 dev: log to current dir when debug_assertions is enabled
- c4e6970 Merge pull request #75 from acheronfail/fix/windows-releases
- cf48568 Merge pull request #74 from acheronfail/fix/readme
- fae10c0 fix: update minimum rust version in README.md

# 0.10.7

Updated dependencies, and fixed windows builds in CI.

- fae131d 0.10.7
- fae1f71 fix: misread the version, only 1 exists
- fae12f43 upd: update actions
- fae14c9 fix: update ci.yml to fix windows builds
- f31dce2 Merge pull request #73 from acheronfail/dep/update
- fae1b59 upd: add tests for cli validation and move test fns behind tests cfg
- fae1600 fix: update windows test
- fae1200 upd: minimum rust version needs to be 1.64.0 for clap
- fae17cc add: debug_assert clap test
- fae1c61 upd: update dependencies
- 6984200 Merge pull request #72 from acheronfail/fix/incorrect-indicator-position
- fae17fa fix: remove test warnings by upgrading insta
- fae159d fix: incorrect indicator position with empty context lines

# 0.10.6

- 5be7287 Merge pull request #69 from acheronfail/release/0.10.6
- 9f50cb7 Merge pull request #68 from acheronfail/fix/ci-and-building-issues
- fae19b9 fix: incorrect rust version in ci.yml
- fae1a87 0.10.6
- fae1b4c upd: update flexi_logger and other dependencies
- fae125f fix: ci and building issues
- fae197c fix: flush keyboard events if drawing to the terminal is slow

# 0.10.5

- 482293e 0.10.5
- 0f38d52 chore: update to clap v3 (finally it is out!)
- fae1661 repo: cargo clippy
- 6006df9 Merge pull request #62 from cpkio/multiple-fix
- fae1d85 fmt
- bef7bb8 Fixed build errors
- 70e751e Merge pull request #60 from acheronfail/fix/indicator-position-full-lines
- fae1d69 fix: indicator position issue with full-length lines

# 0.10.4

- fae103e chore: upgrade dependencies and regenerate lockfile

# 0.10.3

- fae1877 fix: incorrect indicator position for matches on next line

# 0.10.2

- fae12f4 fix: remove more obsolete github actions set env

# 0.10.1

- fae1703 fix: remove obsolete github actions set env

# 0.10.0

- d18c833 Merge pull request #59 from acheronfail/chore/cargo-readme-and-badges
- 8717526 Merge pull request #58 from acheronfail/fix/indicator-position-line-wrapping
- fae1ca2 fix: make moving the indicator position more robust
- fae12c4 docs: use cargo-readme to generate README.md with badges
- fae1a49 refactor: Display trait for PrintableStyle
- fae152b chore: impl Default for PrintableStyle
- 3482b57 Merge pull request #56 from acheronfail/chore/upgrade-dependencies
- fae1444 chore: upgrade dependencies
- fae1fa9 ci: remove scheduled action
- c464d18 Merge pull request #55 from acheronfail/refactor/code-health
- fae1fcf refactor: move Item and SubItem into ui module
- 1b780b9 Merge pull request #54 from acheronfail/feat/handle-too-small-window
- fae18ba feat: handle the case when the terminal window is too small
- 699d5f6 Merge pull request #52 from acheronfail/refactor/performance-issues
- fae1147 refactor: use .replace rather than collecting chars to strings
- fae10a0 fix(app): allow no arguments when reading from a file
- ac740cf Merge pull request #50 from acheronfail/fix/indicator-position
- fae1ab1 fix(ui): various issues with indicator positions
- fae1c7f feat(app): allow reading rg messages from a file
- fae1fd0 fix(ui): change minimum dimensions from 40x40 to 70x20
- fae1d9f refactor: rename rg_results to rg_messages
- e9500f8 Merge pull request #49 from acheronfail/fix/trailing-newlines
- fae1e8a fix(ui): new lines were incorrectly inserted in one line modes
- 879b2e3 Merge pull request #17 from acheronfail/chore/housekeeping
- fae1fb2 fix(tests): use NamedTempFile::keep to address Windows issues
- fae1959 fix(replace): try using tempfile for atomic writing
- efbedd7 Merge pull request #46 from acheronfail/fix/continue-on-error
- a8bcb5e Merge pull request #45 from acheronfail/chore/add-debugging-helpers
- fae1cd6 chore(debug): add logging utilities via RUST_LOG env var
- fae1b42 fix(replace): fallback to UTF-8 rather than ASCII
- fae12bf fix(replace): continue to next file on error
- 4429361 Merge pull request #44 from acheronfail/chore/add-snapshots
- fae1c0a fix(dev): use backtrace feature for insta
- fae128f chore(dev): use insta for snapshot testing
- 3d3d67c Merge pull request #43 from acheronfail/fix/incorrect-line-numbers
- fae1cd8 change(ui): default to PrintableStyle::Hidden
- bebc2b8 Merge pull request #41 from acheronfail/fix/unselected-matches-disappearing
- fae14a9 fix(ui): deselected matches will no longer disappear when confirming
- fae12c6 fix(ui): improve rendering of line numbers with multiline spans
- 74aa955 Merge pull request #40 from acheronfail/fix/control-characters
- fae12fc feat: display CtrlChars in stats line and fix control chars rendering
- fae17bb fix(doc): update incorrect keybinding in manpage
- dc11c8f Merge pull request #34 from acheronfail/fix/wrap-long-lines
- fae1db6 feat: implement wrapping for lines longer than terminal width
- fae1a6f chore(doc): update README.md
- 0aff672 Merge pull request #31 from acheronfail/fix/update-actions
- fae1d0d chore: update justfile
- fae16d9 fix: update deprecated set-env calls and update cross

# 0.9.0

- dd66f4f Merge pull request #30 from acheronfail/feat/allow-multiline-replacements
- fae1c795 feat: allow multiline replacements
- fae1e22 fix(ui): unselected matches now display as context in replacement mode
- fae1386 fix(ui): line numbers were still highlighted after leaving SelectMatches mode
- fae170c refactor: minor tweaks to tests and variable naming
- fae1263 chore: update CHANGELOG.md

# 0.8.0

- fae1f09 fix: update pinned rust version
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

# 0.7.5

- fae1a3d fix(dev): fixup justfile bump command
- fae1011 fix: re-word keybinding menu and fix formatting

# 0.7.4

- fae162f fix(ui): allow scrolling the text in the help view
- fae141b fix(ui): maintain newlines in help menu

# 0.7.3

- fae1d59 fix(fmt): build.rs
- fae18c8 fix(ci): recognise tags without preceding "v"
- fae1350 dev: add a justfile for common commands
- fae1304 fix: issues with an old Cargo.lock and beta versions of clap

# v0.7.2

- fae10d8 fix: crate a post-action that publishes the crate after other actions

# v0.7.1

- fae10ec fix: add step to publish crate in CI
- fae11ed feat: style select mode more clearly
- fae140b fix: handle case when rg returns no matches
- fae16ee refactor: abstract reading rg messages into another module
- fae1b2a feat: replace non-printable whitespace with symbols
- fae1135 chore: update README.md with png rather than gif

# v0.7.0

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

# v0.6.0

- fae1f96 docs: update README and manpage documentation for encoding
- fae1bd2 chore: add tests for replacing with shorter text
- fae1ae6 refactor: minor change to drop file bytes sooner
- fae1729 fix: add workaround for UTF8 with BOM
- fae1c962 feat: properly support UTF16 encodings

# v0.5.0

- fae12cf refactor: create cli mod to split arg definition and validation
- fae19a0 refactor: major refactor to CLI parsing to sniff rg arguments
- fae1166 Revert "refactor: minor fixup for Windows systems"
- fae1af0 refactor: minor fixup for Windows systems
- fae1377 fix: filter out more invalid rg args and update manpage

# v0.4.10

- fae1a06 fix: improved error handling when running ripgrep
- fae1ab83 fix: continue on error rather than bailing out

# v0.4.6 - v0.4.9

- Various issues at manpage generation and testing out CI

# v0.4.5

- fae1510 ci: update github action build names
- fae1e3f chore: merge help.txt and manpage generation
- fae1014 ci: add x86_64-unknown-linux-gnu target to release actions

# v0.4.4

- fae144a chore: remove final new ci testing configuration
- fae1a66 chore: move ci2 dir over old ci
- fae10f9 ci: switch from TravisCI to GitHub Actions
- fae1293 fix: issues with initial Windows builds
- fae13a0 chore: rustfmt

# v0.4.3

- fae1c56 fix: return errors running rg without a panic

# v0.4.2

- fae1a7e fix: better error handling for when ripgrep is not installed
- fae166f doc: update README.md

# v0.4.1

- fae13be travis: fix issue deploying GitHub releases
- fae1a99 chore: write tests for detecting rg encoding
- fae1169 ui: add an extra newline before final ReplacementResult stats

# v0.4.0

- fae193a feat: read encoding passed to rg and use that as override
- fae1015 refactor: only pass rg args to run_ripgrep()

# v0.3.3

- fae103d fix: alignment issues rendering non UTF8 items as Text structs
- fae15c4 chore: handle non UTF8 paths when converting Item to Text
- fae12f5 chore: make demo.gif smaller and easier to see

# v0.3.2

- fae18c6 doc: update demo.gif
- fae1bb2 fix: incorrect number of reported matches in results list
- fae1ebe doc: swap mp4 for gif so it works in the preview
- fae14fc doc: add demo recording and add it to the README
- fae10ce fix: handle UTF-8 BOM if found
- fae1059 refactor: simplify and improve the replacement results

# v0.3.1

- fae1138 feat: show <empty> in main view when replacement string is empty
- fae178b chore: update README

# v0.3.0

- fae1f65 feat: support encodings other than UTF-8
- fae13d0 refactor: simplify temp_rg_msg helper fn
- fae1043 refactor: simplify test by sharing variables
- fae167a chore: add tests to ensure only Match items are replaced
- fae1fa3 travis: fix issue with multiple builds trying to publish to crates
- fae1d31 chore: sort dependencies alphabetically in Cargo.toml

# v0.2.0

- fae19b2 fix: issue when replacing multiple matches in a single file
- fae1292 feat: return and log a replacement result
- fae12f1 fix: return error when no arguments are provided

# v0.1.1

- fae16e8 fix: issue replacing matches with an offset > 0
- fae1995 chore: update comment
- fae1781 chore: tests for perform_replacements() and many test utilities
- fae1275 fix: use correct Apache-2.0 SPDX identifier
- fae1ce0 fix: only have 5 keywords in Cargo.toml
- fae1597 chore: add homepage and repository to Cargo.toml
- fae15f5 travis: add initial configuration

# v0.1.0

Initial release