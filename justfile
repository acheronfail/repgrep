set positional-arguments

badge-crates := "[![crate](https://img.shields.io/crates/v/repgrep)](https://crates.io/crates/repgrep)"
badge-docs := "[![documentation](https://docs.rs/repgrep/badge.svg)](https://docs.rs/repgrep)"
bench-json := "benches/rg.json"

# run this once after you pull down the repository
setup:
    cargo install cargo-bump
    cargo install cargo-readme
    if   command -v pacman  >/dev/null 2>&1 /dev/null; then sudo pacman -S --needed ripgrep; fi
    if   command -v apt-get >/dev/null 2>&1 /dev/null; then sudo apt-get install ripgrep; fi
    if ! command -v rg      >/dev/null 2>&1 /dev/null; then echo "please install rg!"; exit 1; fi

# run rgr locally with logging enabled - use `just devlogs` to view output
dev *args:
    RUST_LOG=trace cargo run -- "$@"

# follows logs from `just dev`
dev-logs:
    tail -f ./rgr.log

# ensures that data is available for the benchmarks
setup-bench:
    if [ ! -f "{{bench-json}}" ]; then rg --json --no-config . ./ > "{{bench-json}}"; fi

# run the benchmarks
bench: setup-bench
    cargo bench

# update the readme
readme:
    printf "%s\n%s\n%s" "{{ badge-crates }}" "{{ badge-docs }}" "$(cargo readme)" > README.md
    sed -i 's/# repgrep/# repgrep (rgr)/' README.md

check-dirty:
    if [ ! -z "$(git status --porcelain)" ]; then \
        echo "It seems there are uncommitted changes, please run this command in a clean git state"; \
        exit 1; \
    fi \

# Bumps the crate,a creates a tag and commits the changed files
bump +TYPE: check-dirty
    #!/usr/bin/env bash
    set -euxo pipefail

    last_tag=$(git describe --tags | grep -oEm 1 '([0-9]+\.[0-9]+\.[0-9]+)')
    commits=$(git log --no-decorate --oneline "$last_tag"..HEAD | sed 's/^/- /')

    cargo fmt
    cargo bump {{ TYPE }}
    cargo check

    just readme

    version=$(grep -oEm 1 '([0-9]+\.[0-9]+\.[0-9]+)' Cargo.toml)
    printf '# %s\n\n%s\n\n%s' "$version" "$commits" "$(cat CHANGELOG.md)" > CHANGELOG.md

    git add .
    git commit -v -m "$version"
    git tag "$version"
