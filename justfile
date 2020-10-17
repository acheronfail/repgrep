# Bumps the crate, creates a tag and commits the new Cargo.lock file
# Requires https://github.com/wraithan/cargo-bump
bump +TYPE:
	#!/usr/bin/env bash
	last_tag=$(git describe --tags | grep -oEm 1 '([0-9]+\.[0-9]+\.[0-9]+)')
	commits=$(git log --no-decorate --oneline "$last_tag"..HEAD | sed 's/^/- /')

	cargo fmt
	cargo bump {{ TYPE }}
	cargo check

	version=$(grep -oEm 1 '([0-9]+\.[0-9]+\.[0-9]+)' Cargo.toml)
	printf '# %s\n\n%s\n\n%s' "$version" "$commits" "$(cat CHANGELOG.md)" > CHANGELOG.md

	git add Cargo.toml Cargo.lock CHANGELOG.md
	git commit -v -m "$version"
	git tag "$version"
