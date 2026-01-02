//! html2json - HTML to JSON extractor using html5ever
//!
//! A Rust port of cheerio-json-mapper using html5ever for HTML parsing.
//!
//! # Overview
//!
//! This library extracts structured JSON data from HTML using CSS selectors
//! defined in a JSON spec format.
//!
//! # Basic Example
//!
//! ```no_run
//! use html2json::{extract, Spec};
//!
//! let html = r#"<html><body><h1>Hello</h1><p class="desc">World</p></body></html>"#;
//! let spec_json = r#"{"title": "h1", "description": "p.desc"}"#;
//! let spec: Spec = serde_json::from_str(spec_json)?;
//! let result = extract(html, &spec)?;
//! assert_eq!(result["title"], "Hello");
//! assert_eq!(result["description"], "World");
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod dom;
pub mod pipe;
pub mod spec;

pub use dom::Dom;
pub use spec::Spec;

use anyhow::Result;

/// Extract JSON from HTML using a spec
///
/// # Arguments
///
/// * `html` - The HTML source to parse
/// * `spec` - The extraction specification
///
/// # Example
///
/// ```
/// use html2json::{extract, Spec};
///
/// let html = r#"<div class="item"><span>Price: $25.00</span></div>"#;
/// let spec_json = r#"{"price": ".item span | regex:\\$(\\d+\\.\\d+)"}"#;
/// let spec: Spec = serde_json::from_str(spec_json)?;
/// let result = extract(html, &spec)?;
/// assert_eq!(result["price"], "25.00");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn extract(html: &str, spec: &Spec) -> Result<serde_json::Value> {
    let dom = Dom::parse(html)?;
    dom.extract(spec)
}

#[cfg(test)]
mod tests {
    use crate::extract;
    use crate::spec::Spec;
    const HTML: &str = include_str!("../examples/hn.html");

    #[test]
    fn basic_text_extraction() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "title",
                "title": "$"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["title"], "Hacker News");
    }

    #[test]
    fn attribute_extraction() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "rss_link": "link[rel=alternate] | attr:href"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["rss_link"], "rss");
    }

    #[test]
    fn scoping_with_dollar() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": ".pagetop",
                "first_link": "a"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["first_link"], "Hacker News");
    }

    #[test]
    fn nested_scoping() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "head",
                "head_element": {
                    "$": "link",
                    "href": "$ | attr:href",
                    "rel": "$ | attr:rel"
                }
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(
            result["head_element"]["href"],
            "news.css?fFlkMoHAedK8lfBWEYBd"
        );
        assert_eq!(result["head_element"]["rel"], "stylesheet");
    }

    #[test]
    fn collection_extraction() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "ranks": [{
                    "$": ".rank",
                    "value": "$"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let arr = result["ranks"].as_array().unwrap();
        assert!(arr.len() >= 3);
        assert_eq!(arr[0]["value"], "1.");
        assert_eq!(arr[1]["value"], "2.");
        assert_eq!(arr[2]["value"], "3.");
    }

    #[test]
    fn collection_with_nested_properties() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "items": [{
                    "$": "tr.athing",
                    "id": "$ | attr:id",
                    "title": ".titleline a"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let arr = result["items"].as_array().unwrap();
        assert!(arr.len() >= 2);
        assert_eq!(arr[0]["id"], "46446815");
        assert_eq!(arr[0]["title"], "I canceled my book deal");
    }

    #[test]
    fn literal_values() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "source": "'html2json'",
                "version": 1.5,
                "active": true,
                "data": null
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["source"], "html2json");
        assert_eq!(result["version"], 1.5);
        assert_eq!(result["active"], true);
        assert!(result["data"].is_null());
    }

    #[test]
    fn trim_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "title": "title | trim"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["title"], "Hacker News");
    }

    #[test]
    fn lowercase_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "title_lower": "title | lower"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["title_lower"], "hacker news");
    }

    #[test]
    fn uppercase_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "title_upper": "title | upper"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["title_upper"], "HACKER NEWS");
    }

    #[test]
    fn substring_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "partial": "title | substr:0:6"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["partial"], "Hacker");
    }

    #[test]
    fn parse_as_number_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#hnmain",
                "table_width": "$ | attr:width | regex:(\\d+) | parseAs:int"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["table_width"], 85);
    }

    #[test]
    fn regex_pipe() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "points": ".score | regex:(\\d+)\\s*points"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["points"], "156");
    }

    #[test]
    fn no_match_returns_null() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "missing": ".nonexistent-element",
                "present": "title"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert!(result["missing"].is_null());
        assert_eq!(result["present"], "Hacker News");
    }

    #[test]
    fn empty_collection_returns_empty_array() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "items": [{
                    "$": ".nonexistent",
                    "value": "$"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let arr = result["items"].as_array().unwrap();
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn multiple_attributes() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "lang": "html | attr:lang",
                "page_title": "title"
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        assert_eq!(result["lang"], "en");
        assert_eq!(result["page_title"], "Hacker News");
    }

    #[test]
    fn complex_nested_structure() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#hnmain",
                "submissions": [{
                    "$": "tr.athing",
                    "id": "$ | attr:id",
                    "title": ".titleline a"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let items = result["submissions"].as_array().unwrap();
        assert!(items.len() >= 1);
        assert_eq!(items[0]["id"], "46446815");
        assert_eq!(items[0]["title"], "I canceled my book deal");
    }

    #[test]
    fn self_selector_in_collection() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "titles": [{
                    "$": ".titleline a",
                    "text": "$"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let arr = result["titles"].as_array().unwrap();
        assert!(arr.len() >= 2);
        assert_eq!(arr[0]["text"], "I canceled my book deal");
    }

    #[test]
    fn next_sibling_selector() {
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "#hnmain",
                "items": [{
                    "$": "tr.athing",
                    "title": ".titleline a",
                    "score": "+ .subtext .score"
                }]
            }"##,
        )
        .unwrap();
        let result = extract(HTML, &spec).unwrap();
        let items = result["items"].as_array().unwrap();
        assert!(items.len() >= 1);
        assert_eq!(items[0]["title"], "I canceled my book deal");
        assert_eq!(items[0]["score"], "156 points");
    }

    #[test]
    fn void_pipe() {
        let rss_xml = include_str!("../examples/rss.xml");
        // The void pipe should work regardless of its position in the pipe chain
        let spec: Spec = serde_json::from_str(
            r##"{
                "$": "channel",
                "link_trimmed": "link | void | trim",
                "link_lower": "link | void | lower"
            }"##,
        )
        .unwrap();
        let result = extract(rss_xml, &spec).unwrap();
        assert_eq!(result["link_trimmed"], "https://example.com");
        assert_eq!(result["link_lower"], "https://example.com");
    }

    #[test]
    fn rss_feed_extraction() {
        let rss_xml = include_str!("../examples/rss.xml");
        let spec_json = include_str!("../examples/rss.json");
        let expected_json = include_str!("../examples/rss.expected.json");

        let spec: Spec = serde_json::from_str(spec_json).unwrap();
        let expected: serde_json::Value = serde_json::from_str(expected_json).unwrap();
        let result = extract(rss_xml, &spec).unwrap();

        similar_asserts::assert_serde_eq!(expected, result);
    }

    #[test]
    fn hackernews_extraction() {
        let spec_json = include_str!("../examples/hn.json");
        let expected_json = include_str!("../examples/hn.expected.json");

        let spec: Spec = serde_json::from_str(spec_json).unwrap();
        let expected: serde_json::Value = serde_json::from_str(expected_json).unwrap();
        let result = extract(HTML, &spec).unwrap();

        similar_asserts::assert_serde_eq!(expected, result);
    }
}
