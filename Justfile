
help:
	@just --list

get-latest-nightly-version:
	@curl -s https://static.rust-lang.org/dist/channel-rust-nightly.toml \
		| sed -n 's/^date[[:space:]]*=[[:space:]]*"\(.*\)"/nightly-\1/p' \
		| head -n 1

set-nightly-version version:
	@sed -i '' "s/^channel = \".*\"/channel = \"{{version}}\"/" rust-toolchain.toml

check-for-crate-updates:
	@cargo outdated --workspace --root-deps-only

update-crates-within-range:
	cargo update

update-crates-compatible:
	cargo upgrade

update-crates-incompatible:
	cargo upgrade --incompatible

lint:
	@cargo clippy -p trmrs_core --no-deps -- -D warnings
	@cargo clippy -p cli --no-deps -- -D warnings
	@(cd device && cargo clippy --no-deps -- -D warnings)

lint-fix:
	@cargo clippy -p trmrs_core --fix --allow-dirty --allow-staged -Z unstable-options -- -D warnings
	@cargo clippy -p cli --fix --allow-dirty --allow-staged -Z unstable-options -- -D warnings
	@(cd device && cargo clippy --fix --allow-dirty --allow-staged -Z unstable-options -- -D warnings)
