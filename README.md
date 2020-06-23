# `repgrep`: An interactive replacer for `ripgrep`.

This is an interactive command line tool to make find and replacement easy.
It uses [`ripgrep`] to find, and then provides you with a simple interface to see
the replacements in real-time and conditionally replace matches.

**DISCLAIMER**: This project doesn't have extensive tests and until it's tested against multiple different encodings and strings, use it at your own risk!

## Usage

After installing, just use `rgr` (think: `rg` + `replace`).

The arguments are:

```bash
rgr <rg arguments> # See `rgr --help` for more details
```

![demo using rgr](./doc/demo.mp4)

## Installation

#### Precompiled binaries

See the [releases] page for pre-compiled binaries.

#### Via Cargo

```bash
cargo install repgrep
```

#### From Source (via Cargo)

```bash
git clone https://github.com/acheronfail/repgrep/
cd repgrep
cargo install --path .
```

[`ripgrep`]: https://github.com/BurntSushi/ripgrep
[releases]: https://github.com/acheronfail/repgrep/releases
