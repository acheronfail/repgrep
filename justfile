# Bumps the crate, creates a tag and commits the new Cargo.lock file
# Requires https://github.com/wraithan/cargo-bump
bump +TYPE:
	cargo fmt
	cargo bump {{ TYPE }} --git-tag
	cargo check
	git add Cargo.lock
	git commit -v --no-edit --amend
