
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
