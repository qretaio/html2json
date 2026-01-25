# html2json

A Rust port of
[cheerio-json-mapper](https://github.com/denkan/cheerio-json-mapper).

---

## Overview

- **Input:** HTML source + Extractor spec (JSON)
- **Output:** JSON matching the structure defined in the spec
- **Available as:** Rust crate, CLI tool, and WebAssembly npm package

## Installation

### npm / WebAssembly

```bash
npm install @qretaio/html2json
```

### From crates.io (Rust)

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

## Usage

### JavaScript / TypeScript

```javascript
import { extract } from "@qretaio/html2json";

const html = `
  <article class="post">
    <h2>My Article</h2>
    <p class="author">John Doe</p>
    <div class="tags">
      <span>rust</span>
      <span>wasm</span>
    </div>
  </article>
`;

const spec = {
  title: "h2",
  author: ".author",
  tags: [
    {
      $: ".tags span",
      name: "$",
    },
  ],
};

const result = await extract(html, spec);
console.log(result);
// {
//   "title": "My Article",
//   "author": "John Doe",
//   "tags": [{"name": "rust"}, {"name": "wasm"}]
// }
```

### CLI

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

### CLI Options

- `--spec, -s <FILE>` - Path to JSON extractor spec file (required)
- `--check, -c <FILE>` - Compare output against expected JSON file. Exits with 0 if match, 1 if differ (with colored diff).

## Spec Format

The spec is a JSON object where each key defines an output field and each value defines a CSS selector to extract that field.

### Basic Selectors

```json
{
  "title": "h1",
  "description": "p.description"
}
```

### Attributes

```json
{
  "link": "a.main | attr:href",
  "image": "img.hero | attr:src"
}
```

### Pipes (Transformations)

```json
{
  "title": "h1 | trim",
  "slug": "h1 | lower | regex:\\s+-",
  "price": ".price | regex:\\$(\\d+\\.\\d+) | parseAs:int"
}
```

Available pipes:

- `trim` - Trim whitespace
- `lower` - Convert to lowercase
- `upper` - Convert to uppercase
- `substr:start:end` - Extract substring
- `regex:pattern` - Regex capture (first group)
- `parseAs:int` - Parse as integer
- `parseAs:float` - Parse as float
- `attr:name` - Get attribute value
- `void` - Extract from void elements, useful for extracting xml

### Collections (Arrays)

```json
{
  "items": [
    {
      "$": ".item",
      "title": "h2",
      "description": "p"
    }
  ]
}
```

### Scoping (`$` selector)

```json
{
  "$": "article",
  "title": "h1",
  "paragraphs": ["p"]
}
```

### Fallback Operators (`||`)

```json
{
  "title": "h1.main || h1.fallback || h1"
}
```

### Optional Fields (`?`)

```json
{
  "title": "h1",
  "description?": "p.description"
}
```

Optional fields that return `null` are removed from the output.

## LICENSE

MIT
