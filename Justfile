lint:
	cargo clippy
	cargo test

run html spec:
    cargo run --bin html2json --features=cli -- {{html}} {{spec}}

# Install the html2json binary
install:
	cargo install --path . --features cli

# Publish the html2json library to crates.io
publish:
	cargo publish

# Dry run publish to check what will be published
publish-dry:
	cargo publish --dry-run
