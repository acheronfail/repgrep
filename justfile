# Requires https://github.com/wraithan/cargo-bump
bump +TYPE:
	cargo bump {{ TYPE }} --git-tag
	cargo check
