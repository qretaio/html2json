lint:
	cargo clippy
	just test
test:
	cargo test
	just check examples/hn.html examples/hn.json examples/hn.expected.json
	just check examples/rss.xml examples/rss.json examples/rss.expected.json

run html spec:
    cargo run --bin html2json --features=cli -- {{html}} {{spec}}

# Check extraction against expected output
check html spec expected:
    cargo run --bin html2json --features=cli -- {{html}} {{spec}} --check {{expected}}

# Install the html2json binary
install:
	cargo install --path . --features cli

# Publish the html2json library to crates.io
publish:
	cargo publish

# Dry run publish to check what will be published
publish-dry:
	cargo publish --dry-run
