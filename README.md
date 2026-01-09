# html2json

A Rust port of
[cheerio-json-mapper](https://github.com/denkan/cheerio-json-mapper).

---

## Overview

- **Input:** HTML source (file path or stdin) + Extractor spec (JSON file)
- **Output:** JSON matching the structure defined in the spec

## Installation

### From crates.io

```bash
cargo install html2json --features cli
```

### From source

```bash
cargo install --path . --features cli
# or from a git repository
cargo install --git https://github.com/qretaio/html2json --features cli
```

### Using just

```bash
just install
```

## Examples

```bash
# Extract from file
html2json examples/hn.html --spec examples/hn.json

# Extract from stdin (pipe from curl)
curl -s https://news.ycombinator.com/ | html2json --spec examples/hn.json

# Extract from stdin (pipe from cat)
cat examples/hn.html | html2json --spec examples/hn.json

# Check output matches expected JSON (useful for testing/CI)
html2json examples/hn.html --spec examples/hn.json --check expected.json
```

### Options

- `--spec, -s <FILE>` - Path to JSON extractor spec file (required)
- `--check, -c <FILE>` - Compare output against expected JSON file. Exits with 0 if match, 1 if differ (with colored diff).

## LICENSE

MIT
