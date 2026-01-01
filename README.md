# html2json

A Rust port of
[cheerio-json-mapper](https://github.com/denkan/cheerio-json-mapper).

---

## Overview

- **Input:** HTML source (URL or file path) + Extractor spec (JSON file)
- **Output:** JSON matching the structure defined in the spec

## Examples

```bash
html2json "https://news.ycombinator.com/" examples/hn.json
html2json examples/hn.html examples/hn.json
```

## LICENSE

MIT
