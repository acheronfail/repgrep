badge-crates := "[![crate](https://img.shields.io/crates/v/repgrep)](https://crates.io/crates/repgrep)"
badge-docs := "[![documentation](https://docs.rs/repgrep/badge.svg)](https://docs.rs/repgrep)"
bench-json := "benches/rg.json"

setup-bench:
    if [ ! -f "{{bench-json}}" ]; then rg --json --no-config . ./ > "{{bench-json}}"; fi

bench: setup-bench
    cargo bench

readme:
	printf "%s\n%s\n%s" "{{ badge-crates }}" "{{ badge-docs }}" "$(cargo readme)" > README.md
	sed -i 's/# repgrep/# repgrep (rgr)/' README.md

# Bumps the crate,a creates a tag and commits the changed files
# Requires https://github.com/wraithan/cargo-bump
bump +TYPE:
    #!/usr/bin/env bash
    if [ ! -z "$(git status --porcelain)" ]; then
        echo "It seems there are uncommitted changes, please run this command in a clean git state"
        exit 1
    fi

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
